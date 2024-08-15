use rustler::{Atom, Binary, Env, NifTaggedEnum, OwnedBinary, Resource, ResourceArc};
use sctp_proto::{
    Association, AssociationHandle, ClientConfig, DatagramEvent, Endpoint, EndpointConfig,
    Error as ProtoError, Event as SctpEvent, Payload, PayloadProtocolIdentifier, ServerConfig,
    StreamEvent, Transmit,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Instant;

mod atoms {
    rustler::atoms! {
        ok,
        error,
        not_connected,
        already_exists,
        unable_to_create,
        invalid_id,
        unable_to_stop,
        unable_to_send,
    }
}

const FAKE_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 5000);

#[derive(NifTaggedEnum)]
enum AtomResult {
    Ok,
    Error(Atom),
}

struct SctpState {
    endpoint: Endpoint,
    assoc: Option<Association>,
    handle: AssociationHandle,
    streams: Vec<u16>,
    last_timeout: Option<Instant>,
}

struct SctpResource {
    state: Mutex<SctpState>,
}

#[rustler::resource_impl]
impl Resource for SctpResource {}

#[derive(NifTaggedEnum)]
enum Event<'a> {
    None,
    Connected,
    Disconnected,
    StreamOpened(u16),
    StreamClosed(u16),
    Data(u16, u32, Binary<'a>),
    Transmit(Vec<Binary<'a>>),
    Timeout(Option<u128>),
}

#[rustler::nif]
fn new() -> ResourceArc<SctpResource> {
    let config = EndpointConfig::new().into();
    // NOTICE: without server_config, endpoint.handle crashes on new remote association
    // make an issue?
    let server_config = ServerConfig::new().into();
    let endpoint = Endpoint::new(config, Some(server_config));
    let state = Mutex::new(SctpState {
        endpoint,
        assoc: None,
        handle: AssociationHandle(0),
        streams: Vec::new(),
        last_timeout: None,
    });

    ResourceArc::new(SctpResource { state })
}

#[rustler::nif]
fn connect(resource: ResourceArc<SctpResource>) -> Atom {
    let mut state = resource.state.lock().expect("Unable to obtain the lock");
    let (handle, assoc) = state
        .endpoint
        .connect(ClientConfig::default(), FAKE_ADDR)
        .expect("Unable to create an associacion");

    state.assoc = Some(assoc);
    state.handle = handle;

    atoms::ok()
}

#[rustler::nif]
fn open_stream(resource: ResourceArc<SctpResource>, id: u16) -> AtomResult {
    let mut state = resource.state.lock().expect("Unable to obtain the lock");

    let SctpState {
        assoc: Some(assoc),
        streams,
        ..
    } = &mut *state
    else {
        return AtomResult::Error(atoms::not_connected());
    };

    if assoc.is_handshaking() || assoc.is_closed() {
        return AtomResult::Error(atoms::not_connected());
    }

    match assoc.open_stream(id, PayloadProtocolIdentifier::Unknown) {
        Ok(_) => {
            streams.push(id);
            AtomResult::Ok
        }
        Err(ProtoError::ErrStreamAlreadyExist) => AtomResult::Error(atoms::already_exists()),
        Err(_) => AtomResult::Error(atoms::unable_to_create()),
    }
}

#[rustler::nif]
fn close_stream(resource: ResourceArc<SctpResource>, id: u16) -> AtomResult {
    // TODO: something doesnt' work in close_stream
    let mut state = resource.state.lock().expect("Unable to obtain the lock");
    let SctpState {
        assoc: Some(assoc),
        streams,
        ..
    } = &mut *state
    else {
        return AtomResult::Error(atoms::not_connected());
    };

    let Some(idx) = streams.iter().position(|stream_id| *stream_id == id) else {
        return AtomResult::Error(atoms::invalid_id());
    };

    streams.remove(idx);

    let Ok(mut stream) = assoc.stream(id) else {
        return AtomResult::Error(atoms::invalid_id());
    };

    let Ok(_) = stream.finish() else {
        return AtomResult::Error(atoms::unable_to_stop());
    };

    let Ok(_) = stream.stop() else {
        return AtomResult::Error(atoms::unable_to_stop());
    };

    AtomResult::Ok
}

#[rustler::nif]
fn send(resource: ResourceArc<SctpResource>, id: u16, ppi: u32, data: Binary) -> AtomResult {
    let mut state = resource.state.lock().expect("Unable to obtain the lock");
    let SctpState {
        assoc: Some(assoc), ..
    } = &mut *state
    else {
        return AtomResult::Error(atoms::not_connected());
    };

    let Ok(mut stream) = assoc.stream(id) else {
        return AtomResult::Error(atoms::invalid_id());
    };

    let bytes: Box<[u8]> = Box::from(data.as_slice());
    let Ok(_) = stream.write_sctp(&bytes.into(), ppi.into()) else {
        return AtomResult::Error(atoms::unable_to_send());
    };

    AtomResult::Ok
}

#[rustler::nif]
fn handle_data(resource: ResourceArc<SctpResource>, data: Binary) -> Atom {
    let mut state = resource.state.lock().expect("Unable to obtain the lock");
    let bytes: Box<[u8]> = Box::from(data.as_slice());

    let res = state
        .endpoint
        .handle(Instant::now(), FAKE_ADDR, None, None, bytes.into());

    let Some((handle, event)) = res else {
        return atoms::ok();
    };

    match event {
        DatagramEvent::NewAssociation(assoc) => {
            // TODO: check if assoc does not already exist
            state.assoc = Some(assoc);
            state.handle = handle;
        }
        DatagramEvent::AssociationEvent(event) => {
            state
                .assoc
                .as_mut()
                .expect("Valid association")
                .handle_event(event);
        }
    };

    atoms::ok()
}

#[rustler::nif]
fn handle_timeout(resource: ResourceArc<SctpResource>) -> AtomResult {
    let mut state = resource.state.lock().expect("Unable to obtain the lock");
    let now = Instant::now();

    let SctpState {
        assoc: Some(assoc),
        endpoint,
        handle,
        ..
    } = &mut *state
    else {
        return AtomResult::Error(atoms::not_connected());
    };

    assoc.handle_timeout(now);

    while let Some(endpoint_event) = assoc.poll_endpoint_event() {
        if let Some(assoc_event) = endpoint.handle_event(*handle, endpoint_event) {
            assoc.handle_event(assoc_event);
        }
    }

    AtomResult::Ok
}

#[rustler::nif]
fn poll(env: Env, resource: ResourceArc<SctpResource>) -> Event {
    let mut state = resource.state.lock().expect("Unable to obtain the lock");
    let now = Instant::now();

    let SctpState {
        assoc: Some(assoc),
        endpoint,
        streams,
        last_timeout,
        ..
    } = &mut *state
    else {
        return Event::None;
    };

    while let Some(transmit) = endpoint.poll_transmit() {
        if let Some(bins) = transmit_to_bins(env, transmit) {
            return Event::Transmit(bins);
        }
    }

    while let Some(transmit) = assoc.poll_transmit(now) {
        if let Some(bins) = transmit_to_bins(env, transmit) {
            return Event::Transmit(bins);
        }
    }

    while let Some(event) = assoc.poll() {
        if let SctpEvent::Connected = event {
            return Event::Connected;
        }

        if let SctpEvent::AssociationLost { .. } = event {
            // TODO: clean up state from assoc
            return Event::Disconnected;
        }

        if let SctpEvent::Stream(stream_event) = event {
            use StreamEvent::*;

            match stream_event {
                Readable { id } | Writable { id } => {
                    // TODO: can id be duplicate?
                    streams.push(id);
                    return Event::StreamOpened(id);
                }
                Stopped { id, .. } | Finished { id } => {
                    if let Some(idx) = streams.iter().position(|stream_id| *stream_id == id) {
                        streams.remove(idx);
                    };
                    return Event::StreamClosed(id);
                }
                _ => {}
            }
        }
    }

    for id in streams {
        let Ok(mut stream) = assoc.stream(*id) else {
            continue;
        };

        let Ok(Some(chunks)) = stream.read() else {
            continue;
        };

        let mut owned = OwnedBinary::new(chunks.len()).expect("Unable to initialize binary");
        chunks
            .read(owned.as_mut_slice())
            .expect("Unable to read incoming data");

        return Event::Data(*id, chunks.ppi as u32, Binary::from_owned(owned, env));
    }

    let timeout = assoc.poll_timeout();
    if timeout != *last_timeout {
        *last_timeout = timeout;
        if let Some(time) = timeout {
            let ms = time.duration_since(now).as_millis();
            return Event::Timeout(Some(ms));
        }
        return Event::Timeout(None);
    }

    Event::None
}

fn transmit_to_bins(env: Env, transmit: Transmit) -> Option<Vec<Binary>> {
    let Payload::RawEncode(raw) = transmit.payload else {
        return None;
    };

    Some(
        raw.into_iter()
            .map(|bytes| {
                let mut owned =
                    OwnedBinary::new(bytes.len()).expect("be able to initialize binary");
                owned
                    .as_mut_slice()
                    .copy_from_slice(bytes.to_vec().as_slice());
                Binary::from_owned(owned, env)
            })
            .collect(),
    )
}

rustler::init!("Elixir.ExSCTP");

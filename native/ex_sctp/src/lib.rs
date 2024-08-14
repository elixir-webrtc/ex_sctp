use sctp_proto::{
    Association, AssociationHandle, ClientConfig, DatagramEvent, Endpoint, EndpointConfig,
    Event as SctpEvent, Payload, PayloadProtocolIdentifier, ServerConfig, StreamEvent, Transmit,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Instant;

use rustler::{Atom, Binary, Env, NifTaggedEnum, OwnedBinary, Resource, ResourceArc};

mod atoms {
    rustler::atoms! {
        ok,
    }
}

const FAKE_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 5000);

struct SctpState {
    endpoint: Endpoint,
    assoc: Option<Association>,
    handle: AssociationHandle,
    streams: Vec<u16>,
    last_now: Instant,
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
    Stream(u16),
    Data(u16, u32, Binary<'a>),
    Transmit(Vec<Binary<'a>>),
    Timeout(u64),
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
        last_now: Instant::now(),
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
fn open_stream(resource: ResourceArc<SctpResource>, id: u16) {
    let mut state = resource.state.try_lock().unwrap();

    let SctpState {
        assoc: Some(assoc),
        streams,
        ..
    } = &mut *state
    else {
        return;
    };

    // TODO verify that we already connected

    // TODO handle errors
    let res = assoc.open_stream(id, PayloadProtocolIdentifier::Unknown);
    // TODO: detect duplicate ids
    streams.push(id);

    if let Err(err) = res {
        println!("Err: {:?}", err);
    }
}

// TODO: close stream?

#[rustler::nif]
fn send(resource: ResourceArc<SctpResource>, id: u16, ppi: u32, data: Binary) {
    let mut state = resource.state.try_lock().unwrap();
    let SctpState {
        assoc: Some(assoc), ..
    } = &mut *state
    else {
        return;
    };

    let Ok(mut stream) = assoc.stream(id) else {
        return;
    };

    let bytes: Box<[u8]> = Box::from(data.as_slice());
    let _ = stream.write_sctp(&bytes.into(), ppi.into());
}

#[rustler::nif]
fn handle_data(resource: ResourceArc<SctpResource>, data: Binary) {
    let mut state = resource.state.try_lock().unwrap();
    // TODO: can we do it without copying?
    let bytes: Box<[u8]> = Box::from(data.as_slice());

    let res = state
        .endpoint
        .handle(Instant::now(), FAKE_ADDR, None, None, bytes.into());

    let Some((handle, event)) = res else {
        return;
    };

    match event {
        DatagramEvent::NewAssociation(assoc) => {
            // TODO: check if we did not already connect
            state.assoc = Some(assoc);
            state.handle = handle;
        }
        DatagramEvent::AssociationEvent(event) => {
            state
                .assoc
                .as_mut()
                .expect("association for the connection")
                .handle_event(event);
        }
    }
}

#[rustler::nif]
fn handle_timeout(resource: ResourceArc<SctpResource>) {
    let now = Instant::now();

    let mut state = resource.state.try_lock().unwrap();
    state.last_now = now;

    let SctpState {
        assoc: Some(assoc),
        endpoint,
        handle,
        ..
    } = &mut *state
    else {
        return;
    };

    assoc.handle_timeout(now);

    while let Some(endpoint_event) = assoc.poll_endpoint_event() {
        if let Some(assoc_event) = endpoint.handle_event(*handle, endpoint_event) {
            assoc.handle_event(assoc_event);
        }
    }
}

#[rustler::nif]
fn poll(env: Env, resource: ResourceArc<SctpResource>) -> Event {
    let mut state = resource.state.try_lock().unwrap();

    let SctpState {
        assoc: Some(assoc),
        endpoint,
        streams,
        last_now,
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

    while let Some(transmit) = assoc.poll_transmit(*last_now) {
        if let Some(bins) = transmit_to_bins(env, transmit) {
            return Event::Transmit(bins);
        }
    }

    while let Some(event) = assoc.poll() {
        if let SctpEvent::Connected = event {
            return Event::Connected;
        }

        if let SctpEvent::Stream(stream_event) = event {
            match stream_event {
                StreamEvent::Readable { id } | StreamEvent::Writable { id } => {
                    // TODO: can id be duplicate?
                    streams.push(id);
                    return Event::Stream(id);
                }
                // TODO: handle other
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

        let mut owned = OwnedBinary::new(chunks.len()).expect("be able to initialize binary");
        let Ok(_) = chunks.read(owned.as_mut_slice()) else {
            continue;
        };

        return Event::Data(*id, chunks.ppi as u32, Binary::from_owned(owned, env));
    }

    // TODO timeouts

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

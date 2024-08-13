use sctp_proto::{
    Association, AssociationHandle, ClientConfig, DatagramEvent, Endpoint, EndpointConfig,
    Event as SctpEvent, Payload, ServerConfig, Transmit,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Instant;

use rustler::{Binary, Env, NifTaggedEnum, OwnedBinary, Resource, ResourceArc};

const FAKE_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 5000);

struct SctpState {
    endpoint: Endpoint,
    assoc: Option<Association>,
    handle: AssociationHandle,
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
    Transmit(Vec<Binary<'a>>),
    Timeout(u64),
}

#[rustler::nif]
fn new() -> ResourceArc<SctpResource> {
    let config = EndpointConfig::new().into();
    // NOTICE: without server_config, endpoint.handle crashes on new association
    // make an issue?
    let server_config = ServerConfig::new().into();
    let endpoint = Endpoint::new(config, Some(server_config));
    let state = Mutex::new(SctpState {
        endpoint,
        assoc: None,
        handle: AssociationHandle(0),
        last_now: Instant::now(),
    });

    ResourceArc::new(SctpResource { state })
}

#[rustler::nif]
fn connect(resource: ResourceArc<SctpResource>) {
    let mut state = resource.state.try_lock().unwrap();
    let (handle, assoc) = state
        .endpoint
        .connect(ClientConfig::default(), FAKE_ADDR)
        .expect("Unable to create associacion");

    state.assoc = Some(assoc);
    state.handle = handle;
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
        last_now,
        ..
    } = &mut *state
    else {
        return Event::None;
    };

    if let Some(transmit) = endpoint.poll_transmit() {
        if let Some(bins) = transmit_to_bins(env, transmit) {
            return Event::Transmit(bins);
        }
    }

    if let Some(transmit) = assoc.poll_transmit(*last_now) {
        if let Some(bins) = transmit_to_bins(env, transmit) {
            return Event::Transmit(bins);
        }
    }

    if let Some(event) = assoc.poll() {
        if let SctpEvent::Connected = event {
            return Event::Connected;
        }

        if let SctpEvent::Stream(_stream_event) = event {
            // TODO
            return Event::None;
        }
    }

    // TODO

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

use std::{fs, mem};
use std::error::Error;
use std::path::Path;

use futures::StreamExt;
use libp2p::{identify, Multiaddr, noise, PeerId, relay, swarm::SwarmEvent, tcp, Transport, yamux};
use libp2p::core::upgrade::Version;
use libp2p::identity::Keypair;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::kad::record::store::MemoryStore;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::{NetworkBehaviour, SwarmBuilder};

use crate::constants::{
    DHT_ED_25529_KEYS_FILE_PATH, DHT_PEER_ID_FILE_PATH, IP_NODE_ONE, TCP_NODE_ONE,
};

mod constants;

//Need to be able to run bootstrap.
const BOOTNODES: &str = "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt";

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    if !Path::new(DHT_PEER_ID_FILE_PATH).exists()
        && !Path::new(DHT_ED_25529_KEYS_FILE_PATH).exists()
    {
        generate_dht_logic();
    }

    let local_public_key = read_ed25519_keypair_from_file();
    let local_peer_id = read_peer_id_from_file();
    println!("Local peer id: {local_peer_id:?}");

    let transport = tcp::tokio::Transport::default()
        .upgrade(Version::V1Lazy)
        .authenticate(noise::NoiseAuthenticated::xx(&local_public_key).unwrap())
        .multiplex(yamux::YamuxConfig::default())
        .timeout(std::time::Duration::from_secs(20))
        .boxed();

    let behaviour = MyBehaviour::new(local_peer_id.clone(), local_public_key.clone());

    let mut swarm =
        SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id.clone()).build();

    swarm
        .listen_on(
            Multiaddr::empty()
                .with(Protocol::Ip4(IP_NODE_ONE))
                .with(Protocol::Tcp(TCP_NODE_ONE)),
        )
        .expect("Can't listen on this address");

    swarm.behaviour_mut().bootstrap_kademlia();

    loop {
        match swarm.next().await.expect("Infinite Stream.") {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening in {address:?}")
            }
            SwarmEvent::Behaviour(event) => {
                println!("{event:?}")
            }
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "MyBehaviourEvent")]
struct MyBehaviour {
    kademlia: Kademlia<MemoryStore>,
    identify: identify::Behaviour,
    relay: relay::Behaviour,
}

impl MyBehaviour {
    fn new(local_peer_id: PeerId, local_public_key: Keypair) -> Self {
        Self {
            kademlia: {
                let store = MemoryStore::new(local_peer_id);
                Kademlia::new(local_peer_id, store)
            },
            identify: {
                let cfg_identify =
                    identify::Config::new("/identify/0.1.0".to_string(), local_public_key.public());
                identify::Behaviour::new(cfg_identify)
            },
            relay: { relay::Behaviour::new(local_peer_id, Default::default()) },
        }
    }

    fn bootstrap_kademlia(&mut self) {
        self.kademlia.add_address(
            &BOOTNODES.parse().expect("Can't parse bootstrap node id."),
            "/dnsaddr/bootstrap.libp2p.io"
                .parse()
                .expect("Can't parse bootstrap node address."),
        );

        self.kademlia.bootstrap().expect("Cant bootstrap");
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum MyBehaviourEvent {
    Kademlia(KademliaEvent),
    Identify(identify::Event),
    Relay(relay::Event),
}

impl From<KademliaEvent> for MyBehaviourEvent {
    fn from(event: KademliaEvent) -> Self {
        MyBehaviourEvent::Kademlia(event)
    }
}

impl From<identify::Event> for MyBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        MyBehaviourEvent::Identify(event)
    }
}

impl From<relay::Event> for MyBehaviourEvent {
    fn from(event: relay::Event) -> Self {
        MyBehaviourEvent::Relay(event)
    }
}

pub fn generate_dht_logic() {
    let ed25519_keys = Keypair::generate_ed25519();
    let peer_id = ed25519_keys.public().to_peer_id();
    write_dht_logic(&peer_id, &ed25519_keys);
}

fn write_dht_logic(peer_id: &PeerId, ed25519_keys: &Keypair) {
    write_peer_id_to_file(peer_id);
    write_ed25519_keypair_to_file(ed25519_keys);
}

fn write_ed25519_keypair_to_file(ed25519_keys: &Keypair) {
    let data: &[u8] = unsafe { structure_as_u8_slice(ed25519_keys) };
    let data_sized = byte_array_to_size_array_keypair(data);
    fs::write(DHT_ED_25529_KEYS_FILE_PATH, *data_sized)
        .expect("Unable to write keypair ed25519 in file");
}

fn write_peer_id_to_file(peer_id: &PeerId) {
    let data: &[u8] = unsafe { structure_as_u8_slice(peer_id) };
    let data_sized = byte_array_to_size_array_peer_id(data);
    fs::write(DHT_PEER_ID_FILE_PATH, *data_sized).expect("Unable to write peer id in file");
}

fn read_ed25519_keypair_from_file() -> Keypair {
    let data: Vec<u8> = fs::read(DHT_ED_25529_KEYS_FILE_PATH).expect("Unable to read file keypair");
    let key_pair_bytes_sized = byte_array_to_size_array_keypair(data.as_slice());
    let key_pair: Keypair = unsafe { mem::transmute_copy(key_pair_bytes_sized) };
    key_pair
}

fn read_peer_id_from_file() -> PeerId {
    let data: Vec<u8> = fs::read(DHT_PEER_ID_FILE_PATH).expect("Unable to read file with peer id");
    let peer_id_bytes_sized = byte_array_to_size_array_peer_id(data.as_slice());
    let peer_id: PeerId = unsafe { mem::transmute_copy(peer_id_bytes_sized) };
    peer_id
}

fn byte_array_to_size_array_keypair(array: &[u8]) -> &[u8; ::std::mem::size_of::<Keypair>()] {
    array.try_into().expect("slice with incorrect length")
}

fn byte_array_to_size_array_peer_id(array: &[u8]) -> &[u8; ::std::mem::size_of::<PeerId>()] {
    array.try_into().expect("slice with incorrect length")
}

unsafe fn structure_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

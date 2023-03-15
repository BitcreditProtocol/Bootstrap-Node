use std::error::Error;
use std::path::Path;
use std::{fs, mem};

use futures::select;
use futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{
    development_transport,
    swarm::{Swarm, SwarmEvent},
    PeerId,
};

use crate::constants::{DHT_ED_25529_KEYS_FILE_PATH, DHT_FOLDER_PATH, DHT_PEER_ID_FILE_PATH};

mod constants;

//Need to be able to run bootstrap.
const BOOTNODES: &str = "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt";

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "MyBehaviourEvent")]
struct MyBehaviour {
    kademlia: Kademlia<MemoryStore>,
    identify: libp2p::identify::Behaviour,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum MyBehaviourEvent {
    Kademlia(KademliaEvent),
    Identify(libp2p::identify::Event),
}

impl From<KademliaEvent> for MyBehaviourEvent {
    fn from(event: KademliaEvent) -> Self {
        MyBehaviourEvent::Kademlia(event)
    }
}

impl From<libp2p::identify::Event> for MyBehaviourEvent {
    fn from(event: libp2p::identify::Event) -> Self {
        MyBehaviourEvent::Identify(event)
    }
}

#[async_std::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    init_folders();
    //We generate peer_id and keypair.
    if !Path::new(DHT_PEER_ID_FILE_PATH).exists()
        && !Path::new(DHT_ED_25529_KEYS_FILE_PATH).exists()
    {
        generate_dht_logic();
    }

    let local_key = read_ed25519_keypair_from_file();
    let key_copy = local_key.clone();
    let local_peer_id = read_peer_id_from_file();

    println!("Local peer id: {local_peer_id:?}");

    let transport = development_transport(local_key).await?;

    let mut swarm = {
        let store = MemoryStore::new(local_peer_id);
        let mut kademlia = Kademlia::new(local_peer_id, store);

        let mut cfg_identify =
            libp2p::identify::Config::new("identify version 1".to_string(), key_copy.public());
        let identify = libp2p::identify::Behaviour::new(cfg_identify);

        let mut behaviour = MyBehaviour { kademlia, identify };
        Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
    };

    swarm
        .behaviour_mut()
        .kademlia
        .add_address(&BOOTNODES.parse()?, "/dnsaddr/bootstrap.libp2p.io".parse()?);

    swarm
        .behaviour_mut()
        .kademlia
        .bootstrap()
        .expect("Cant bootstrap");

    //TODO: static id.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        //TODO: delete this select and use just swarm.select_next_some().await
        select! {
        event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening in {address:?}")
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Identify(libp2p::identify::Event::Received {peer_id, info: _})) => {
                    println!("New node identify.");
                    for address in swarm.behaviour_mut().addresses_of_peer(&peer_id) {
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, address);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Kademlia(KademliaEvent::RoutingUpdated { peer, addresses, is_new_peer: _, bucket_range: _, old_peer: _ })) => {
                    println!("RoutingUpdated");
                    swarm.behaviour_mut().identify.push(std::iter::once(peer));
                    println!("{peer:?}");
                    println!("{addresses:?}")
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Kademlia(KademliaEvent::UnroutablePeer { peer })) => {
                    println!("UnroutablePeer");
                    println!("{peer:?}")
                },
                SwarmEvent::Behaviour(event) => {
                    println!("New event");
                    println!("{event:?}")
                },
            _ => {}
            }
        }
    }
}

fn init_folders() {
    if !Path::new(DHT_FOLDER_PATH).exists() {
        fs::create_dir(DHT_FOLDER_PATH).expect("Can't create folder dht.");
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

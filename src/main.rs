use futures::StreamExt;
use futures::select;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{
    development_transport, identity,
    swarm::{Swarm, SwarmEvent},
    PeerId,
};
use std::error::Error;

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
    //TODO: static values. Special for every device, so need to take from const file.
    let local_key = identity::Keypair::generate_ed25519();
    let key_copy = local_key.clone();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id:?}");

    let transport = development_transport(local_key).await?;

    let mut swarm = {
        let store = MemoryStore::new(local_peer_id);
        let mut kademlia = Kademlia::new(local_peer_id, store);

        let mut cfg_identify = libp2p::identify::Config::new("a".to_string(), key_copy.public());
        let identify = libp2p::identify::Behaviour::new(cfg_identify);

        let mut behaviour = MyBehaviour { kademlia, identify };
        Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
    };

    swarm
        .behaviour_mut()
        .kademlia
        .bootstrap()
        .expect("Cant bootstrap");

    //TODO: static id.
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        select! {
        event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening in {address:?}")
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Identify(libp2p::identify::Event::Received {peer_id, info: _})) => {
                    println!("New node identify.");
                    for address in  swarm.behaviour_mut().addresses_of_peer(&peer_id) {
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

use std::net::Ipv4Addr;

//TODO: normal path.
// Now this is because we have service in other file in server. And when we do smth it is try to manipulate with this folder.
pub const DHT_PEER_ID_FILE_PATH: &str = "/opt/bitcredit/release/dht/peer_id";
pub const DHT_ED_25529_KEYS_FILE_PATH: &str = "/opt/bitcredit/release/dht/ed25519_keys";
// pub const DHT_PEER_ID_FILE_PATH: &str = "dht/peer_id";
// pub const DHT_ED_25529_KEYS_FILE_PATH: &str = "dht/ed25519_keys";

//NODE ONE /ip4/45.147.248.87/tcp/1908
pub const IP_NODE_ONE: Ipv4Addr = Ipv4Addr::new(45, 147, 248, 87);
pub const TCP_NODE_ONE: u16 = 1908;

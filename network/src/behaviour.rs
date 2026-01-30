use libp2p::{
    gossipsub, kad, mdns, swarm::NetworkBehaviour,
};

#[derive(NetworkBehaviour)]
pub struct RhizomeBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
}

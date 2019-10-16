use std::env;
use futures::{prelude::*, future};
use libp2p::{PeerId, Transport};
use libp2p::tcp::TcpConfig;
use libp2p::identity::Keypair;
use libp2p::mplex::MplexConfig;
use libp2p::core::upgrade::SelectUpgrade;
use libp2p::core::nodes::network::Network;
use libp2p::core::transport::upgrade::{Builder, Version};
use libp2p::yamux;
//use libp2p_secio as secio;
use libp2p::Swarm;
use libp2p::secio::SecioConfig;
use libp2p::swarm::protocols_handler::{ProtocolsHandler, NodeHandlerWrapper, SubstreamProtocol, DummyProtocolsHandler};
use libp2p::swarm::toggle::Toggle;

fn main() {
    env_logger::init();

    let local_key = Keypair::generate_ed25519();
    let local_public_key = local_key.public();

    let transport = TcpConfig::new()
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(local_key))
        .multiplex(MplexConfig::new());

    //let swarm = Swarm::new(transport, DummyProtocolsHandler::default(), local_public_key);
}

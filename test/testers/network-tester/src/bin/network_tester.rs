use std::env;
use futures::{prelude::*, future};
use libp2p::{PeerId, Multiaddr, Transport};
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
use libp2p::swarm::NetworkBehaviour;
use libp2p::mdns::Mdns;
use libp2p::mplex::Substream;

fn main() {
    env_logger::init();

    let local_key = Keypair::generate_ed25519();
    let local_public_key = local_key.public();

    let transport = TcpConfig::new();

    //let node: Multiaddr = "/ip4/188.62.22.15/tcp/30333/p2p/Qmd6oSuC4tEXHxewrZNdwpVhn8b4NwxpboBCH4kHkH1EYb".parse().unwrap();
    let node: Multiaddr = "/ip4/127.0.0.1/tcp/30333".parse().unwrap();

    let mut conn = transport.dial(node).unwrap();

    // Kick it off
    tokio::run(futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            //match transport.clone().dial(node.clone()).unwrap().poll().unwrap() {
            match conn.poll().unwrap() {
                Async::Ready(x) => println!("{:?}", x),
                Async::NotReady => break,
            }
            break;
        }

        Ok(Async::NotReady)
    }));
}

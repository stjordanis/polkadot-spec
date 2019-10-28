#[macro_use]
extern crate libp2p;
#[macro_use]
extern crate log;

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
use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourEventProcess};
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::mplex::Substream;
use tokio::io::{AsyncRead, AsyncWrite};
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::mdns::service::{MdnsPacket, MdnsService};
use libp2p::kad::record::store::MemoryStore;
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::kad::protocol::KadRequestMsg;
use libp2p::ping::{Ping, PingEvent};
use libp2p::ping::handler::PingConfig;

fn main() {
    env_logger::init();

    #[derive(NetworkBehaviour)]
    struct MyBehaviour<TSubstream: AsyncRead + AsyncWrite> {
        kademlia: Kademlia<TSubstream, MemoryStore>,
        mdns: Mdns<TSubstream>,
        identify: Identify<TSubstream>,
        ping: Ping<TSubstream>,
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour<TSubstream> {
        fn inject_event(&mut self, event: MdnsEvent) {
            match event {
                MdnsEvent::Discovered(addrs) => {
                    for (peer_id, multi_addr) in addrs {
                        debug!("Discovered {}/{}", &multi_addr, peer_id);
                        self.kademlia.add_address(&peer_id, multi_addr);
                    }
                },
                e @ _ => {
                    warn!("Received unhandled mDNS event: {:?}", e);
                },
            }
        }
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<KademliaEvent> for MyBehaviour<TSubstream> {
        fn inject_event(&mut self, event: KademliaEvent) {
            match event {
                KademliaEvent::RoutingUpdated{peer, addresses, ..} => {
                    info!("Updated DHT with address of {}:", peer);
                    for address in addresses.iter() {
                        info!("> {}", address);
                    }
                },
                KademliaEvent::BootstrapResult(bootstrap_result) => {
                    if let Ok(result) = bootstrap_result {
                        debug!("Bootstrap result: {:?}", result);
                    }
                },
                KademliaEvent::Discovered{peer_id, addresses, ..} => {
                    info!("Discovered from DHT peer {}:", peer_id);
                    for address in addresses.iter() {
                        debug!("> {}", address);
                    }
                },
                e @ _ => {
                    warn!("Received unhandled Kademlia event: {:?}", e);
                },
            }
        }
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent> for MyBehaviour<TSubstream> {
        fn inject_event(&mut self, event: IdentifyEvent) {
            match event {
                _ => info!("Identity event: {:?}", event),
            }
        }
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<PingEvent> for MyBehaviour<TSubstream> {
        fn inject_event(&mut self, event: PingEvent) {
            match event {
                _ => info!("Ping event: {:?}", event),
            }
        }
    }

    let local_key = Keypair::generate_ed25519();
    let local_public_key = local_key.public();
    let local_peer_id = PeerId::from_public_key(local_public_key.clone());

    info!("Local node ID: {:?}", local_peer_id);

    let transport = TcpConfig::new()
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(local_key))
        .multiplex(MplexConfig::new());

    let store = MemoryStore::new(local_peer_id.clone());
    let behaviour = Kademlia::new(local_peer_id.clone(), store);
    let mdns = Mdns::new().unwrap();
    let ping = Ping::new(PingConfig::new());

    let behaviour = MyBehaviour {
        kademlia: behaviour,
        mdns: mdns,
        identify: Identify::new(String::from("/substrate/1.0"), String::from(""), local_public_key.clone()),
        ping,
    };

    // remote node
    let remote_peer_id: PeerId = "Qmd6oSuC4tEXHxewrZNdwpVhn8b4NwxpboBCH4kHkH1EYb".parse().unwrap();
    let remote_addr: Multiaddr = "/ip4/127.0.0.1/tcp/30333".parse().unwrap();

    // Create swarm, listen on local port
    let mut swarm = Swarm::new(transport.clone(), behaviour, local_peer_id.clone());
    let _ = Swarm::dial_addr(&mut swarm, remote_addr.clone()).unwrap();
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    let kad_ping = KadRequestMsg::Ping;

    // Run worker
    let mut node_known = false;
    let mut listening = false;
    tokio::run(futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match swarm.poll().unwrap() {
                Async::Ready(_) => {},
                Async::NotReady => {
                    if !listening {
                        if let Some(a) = Swarm::listeners(&swarm).next() {
                            info!("Listening locally on {:?}", a);
                            listening = true;
                        }
                    }
                    break
                }
            }
        }

        if node_known {
            swarm.kademlia.bootstrap();
        } else {
            if swarm.kademlia.kbuckets_entries().count() > 0 {
                node_known = true;
            }
        }
 
        Ok(Async::NotReady)
    }));
}

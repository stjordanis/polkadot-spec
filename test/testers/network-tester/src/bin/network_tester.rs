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

fn main() {
    env_logger::init();

    let local_key = Keypair::generate_ed25519();
    let local_public_key = local_key.public();
    let peer_id = PeerId::from_public_key(local_public_key);

    println!("ID: {:?}", peer_id);

    #[derive(NetworkBehaviour)]
    struct MyBehaviour<TSubstream: AsyncRead + AsyncWrite> {
        kademlia: Kademlia<TSubstream, MemoryStore>,
        mdns: Mdns<TSubstream>,
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour<TSubstream> {
        fn inject_event(&mut self, event: MdnsEvent) {
            match event {
                MdnsEvent::Discovered(addrs) => {
                    for (peer_id, multi_addr) in addrs {
                        info!("Discovered {}/{}", &multi_addr, peer_id);
                        self.kademlia.add_address(&peer_id, multi_addr);
                    }

                },
                _ => {},
            }
        }
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<KademliaEvent> for MyBehaviour<TSubstream> {
        // Called when `floodsub` produces an event.
        fn inject_event(&mut self, event: KademliaEvent) {
            match event {
                KademliaEvent::RoutingUpdated{peer, addresses, ..} => {
                    info!("Updated DHT with {} - {:?}", peer, addresses);
                },
                _ => {},
            }
        }
    }

    // remote node
    let remote_peer_id: PeerId = "Qmd6oSuC4tEXHxewrZNdwpVhn8b4NwxpboBCH4kHkH1EYb".parse().unwrap();
    let remote_addr: Multiaddr = "/ip4/127.0.0.1/tcp/30333".parse().unwrap();

    let transport = TcpConfig::new()
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(local_key))
        .multiplex(MplexConfig::new());

    let store = MemoryStore::new(peer_id.clone());
    let behaviour = Kademlia::new(peer_id.clone(), store);

    let mdns = Mdns::new().unwrap();

    let behaviour = MyBehaviour {
        kademlia: behaviour,
        mdns: mdns,
    };

    let mut swarm = Swarm::new(transport.clone(), behaviour, peer_id);
    let _ = Swarm::dial_addr(&mut swarm, remote_addr.clone()).unwrap();

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    let mut listening = false;
    tokio::run(futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match swarm.poll().unwrap() {
                Async::Ready(x) => {
                    println!("READY: {:?}", x);
                },
                Async::NotReady => {
                    if !listening {
                        if let Some(a) = Swarm::listeners(&swarm).next() {
                            println!("Listening on {:?}", a);
                            listening = true;
                        }
                    }
                    break
                }
            }
        }

        Ok(Async::NotReady)
    }));

    // Kick it off
    /*
    tokio::run(futures::future::poll_fn(move || -> Result<_, ()> {
        let text = "some test data";
        loop {
            let packet = match service.poll() {
                Async::Ready(packet) => packet,
                Async::NotReady => return Ok(Async::NotReady),
            };

            match packet {
                MdnsPacket::Query(query) => {
                    // We detected a libp2p mDNS query on the network. In a real application, you
                    // probably want to answer this query by doing `query.respond(...)`.
                    println!("Detected query from {:?}", query.remote_addr());
                }
                MdnsPacket::Response(response) => {
                    // We detected a libp2p mDNS response on the network. Responses are for
                    // everyone and not just for the requester, which makes it possible to
                    // passively listen.
                    for peer in response.discovered_peers() {
                        println!("Discovered peer {:?}", peer.id());
                        // These are the self-reported addresses of the peer we just discovered.
                        for addr in peer.addresses() {
                            println!(" Address = {:?}", addr);
                        }
                    }
                }
                MdnsPacket::ServiceDiscovery(query) => {
                    // The last possibility is a service detection query from DNS-SD.
                    // Just like `Query`, in a real application you probably want to call
                    // `query.respond`.
                    println!("Detected service query from {:?}", query.remote_addr());
                }
            }

            /*
            match conn.poll().unwrap() {
                Async::Ready(mut stream) => {
                    match stream.poll_write(text.as_bytes()).unwrap() {
                        Async::Ready(x) => {
                            println!("{:?}", x);
                        },
                        Async::NotReady => {
                            println!("Leaving stream");
                            break;
                        }
                    }
                },
                Async::NotReady => {
                    println!("Leaving connection");
                    break;
                },
            }
            break;
            */
        }

        Ok(Async::NotReady)
    }));
    */
}

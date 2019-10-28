#[macro_use]
extern crate libp2p;
#[macro_use]
extern crate log;

use futures::prelude::*;
use libp2p::{PeerId, Multiaddr, Transport};
use libp2p::tcp::TcpConfig;
use libp2p::identity::Keypair;
use libp2p::mplex::MplexConfig;
use libp2p::core::transport::upgrade::Version;
use libp2p::Swarm;
use libp2p::secio::SecioConfig;
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::mdns::{Mdns, MdnsEvent};
use tokio::io::{AsyncRead, AsyncWrite};
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::kad::record::store::MemoryStore;
use libp2p::ping::{Ping, PingEvent};
use libp2p::ping::handler::PingConfig;

fn main() {
    env_logger::init();

    #[derive(NetworkBehaviour)]
    struct MyBehaviour<TSubstream: AsyncRead + AsyncWrite> {
        kademlia: Kademlia<TSubstream, MemoryStore>,
        mdns: Mdns<TSubstream>,
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
    let kademlia = Kademlia::new(local_peer_id.clone(), store);
    let mdns = Mdns::new().unwrap();
    let ping = Ping::new(PingConfig::new());

    let behaviour = MyBehaviour {
        kademlia: kademlia,
        mdns: mdns,
        ping,
    };

    // remote node
    let _remote_peer_id: PeerId = "Qmd6oSuC4tEXHxewrZNdwpVhn8b4NwxpboBCH4kHkH1EYb".parse().unwrap();
    let remote_addr: Multiaddr = "/ip4/127.0.0.1/tcp/30333".parse().unwrap();

    // Create swarm, listen on local port
    let mut swarm = Swarm::new(transport.clone(), behaviour, local_peer_id.clone());
    let _ = Swarm::dial_addr(&mut swarm, remote_addr.clone()).unwrap();
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    // Run worker
    let mut listening = false;
    let mut bootstrap_started = false;
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

        if !bootstrap_started && swarm.kademlia.kbuckets_entries().count() > 0 {
            swarm.kademlia.bootstrap();
            bootstrap_started = true;
        }

        Ok(Async::NotReady)
    }));
}

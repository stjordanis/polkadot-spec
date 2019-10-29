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

use network_tester::{ClientConnection, ClientConnEvent};

fn main() {
    env_logger::init();

    #[derive(NetworkBehaviour)]
    struct MyBehaviour<TSubstream: AsyncRead + AsyncWrite> {
        kademlia: Kademlia<TSubstream, MemoryStore>,
        mdns: Mdns<TSubstream>,
        ping: Ping<TSubstream>,
        conn: ClientConnection<TSubstream>,
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour<TSubstream> {
        fn inject_event(&mut self, event: MdnsEvent) {
            match event {
                MdnsEvent::Discovered(addrs) => {
                    for (peer_id, multi_addr) in addrs {
                        debug!("Discovered with mDNS {}/{}", &multi_addr, peer_id);
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
                    info!("Updated DHT with address {}:", peer);
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
                    info!("Discovered with Kademlia {}:", peer_id);
                    for address in addresses.iter() {
                        debug!("> {}", address);
                        self.kademlia.add_address(&peer_id, address.clone());
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
                _ => debug!("Ping event: {:?}", event),
            }
        }
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<ClientConnEvent> for MyBehaviour<TSubstream> {
        fn inject_event(&mut self, event: ClientConnEvent) {
            match event {
                _ => debug!("Client connection event: {:?}", event),
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
    let mut kademlia = Kademlia::new(local_peer_id.clone(), store);
    let mdns = Mdns::new().unwrap();
    let ping = Ping::new(PingConfig::new());
    let conn = ClientConnection::default();

    // W3F HQ
    let remote_peer_id1: PeerId = "QmQDn7TE6eGE89vtLgzocTj7VgwdPkZUcgsUGRtLrnidhG".parse().unwrap();
    let remote_addr1: Multiaddr = "/ip4/192.168.4.120/tcp/30333".parse().unwrap();
    // Local Gossamer node
    let remote_peer_id2: PeerId = "QmQqpV9ZA9oarV7JgWvhh1DmKep5gGP7TDTjptjCSvswXF".parse().unwrap();
    let remote_addr2: Multiaddr = "/ip4/127.0.0.1/tcp/07001".parse().unwrap();

    kademlia.add_address(&remote_peer_id1, remote_addr1.clone());
    kademlia.add_address(&remote_peer_id2, remote_addr2.clone());

    kademlia.bootstrap();

    // Create custom behaviour
    let behaviour = MyBehaviour {
        kademlia: kademlia,
        mdns: mdns,
        ping,
        conn,
    };

    // Create swarm, listen on local port
    let mut swarm = Swarm::new(transport.clone(), behaviour, local_peer_id.clone());
    let _ = Swarm::dial_addr(&mut swarm, remote_addr1.clone()).unwrap();
    let _ = Swarm::dial_addr(&mut swarm, remote_addr2.clone()).unwrap();
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    // Run worker
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

        trace!("Amount of Kademlia entries: {}", swarm.kademlia.kbuckets_entries().count());

        Ok(Async::NotReady)
    }));
}

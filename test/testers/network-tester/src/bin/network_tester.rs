#[macro_use]
extern crate libp2p;
#[macro_use]
extern crate log;

use futures::{future, prelude::*, Stream};
use libp2p::core::nodes::network::Network;
use libp2p::core::transport::upgrade::{Builder, Version};
use libp2p::core::upgrade::{apply_outbound, SelectUpgrade};
use libp2p::identity::Keypair;
use libp2p::mplex::MplexConfig;
use libp2p::tcp::TcpConfig;
use libp2p::yamux;
use libp2p::{Multiaddr, PeerId, Transport};
use std::env;
//use libp2p_secio as secio;
use libp2p::identify::{Identify, IdentifyEvent};
use libp2p::kad::protocol::KadRequestMsg;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::mdns::service::{MdnsPacket, MdnsService};
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::mplex::Substream;
use libp2p::secio::SecioConfig;
use libp2p::swarm::protocols_handler::{
    DummyProtocolsHandler, NodeHandlerWrapper, ProtocolsHandler, SubstreamProtocol,
};
use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourEventProcess};
use libp2p::Swarm;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    runtime::current_thread::Runtime,
};

use libp2p::ping::{Ping, PingConfig, PingEvent, PingSuccess};

fn main() {
    env_logger::init();

    #[derive(NetworkBehaviour)]
    struct MyBehaviour<TSubstream: AsyncRead + AsyncWrite> {
        kademlia: Kademlia<TSubstream, MemoryStore>,
        mdns: Mdns<TSubstream>,
        identify: Identify<TSubstream>,
    }

    // impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent> for Ping<TSubstream> {
    //     fn inject_event(&mut self, event: PingEvent) {
    //         match event {
    //             _ => info!("Ping: {:?}", event),
    //         }
    //     }
    // }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<MdnsEvent>
        for MyBehaviour<TSubstream>
    {
        fn inject_event(&mut self, event: MdnsEvent) {
            match event {
                MdnsEvent::Discovered(addrs) => {
                    for (peer_id, multi_addr) in addrs {
                        debug!("Discovered {}/{}", &multi_addr, peer_id);
                        self.kademlia.add_address(&peer_id, multi_addr);
                    }
                }
                e @ _ => {
                    warn!("Received unhandled mDNS event: {:?}", e);
                }
            }
        }
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<KademliaEvent>
        for MyBehaviour<TSubstream>
    {
        fn inject_event(&mut self, event: KademliaEvent) {
            match event {
                KademliaEvent::RoutingUpdated {
                    peer, addresses, ..
                } => {
                    info!("Updated DHT with address of {}:", peer);
                    for address in addresses.iter() {
                        info!("> {}", address);
                    }
                }
                KademliaEvent::BootstrapResult(bootstrap_result) => {
                    if let Ok(result) = bootstrap_result {
                        debug!("Bootstrap result: {:?}", result);
                    }
                }
                e @ _ => {
                    warn!("Received unhandled Kademlia event: {:?}", e);
                }
            }
        }
    }

    impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent>
        for MyBehaviour<TSubstream>
    {
        fn inject_event(&mut self, event: IdentifyEvent) {
            match event {
                _ => info!("IDEN: {:?}", event),
            }
        }
    }

    let local_key = Keypair::generate_ed25519();
    let local_public_key = local_key.public();
    let local_peer_id = PeerId::from_public_key(local_public_key.clone());

    info!("Local node ID: {:?}", local_peer_id);

    let transport = TcpConfig::new()
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(local_key.clone()))
        .multiplex(MplexConfig::new());

    let store = MemoryStore::new(local_peer_id.clone());
    let behaviour = Kademlia::new(local_peer_id.clone(), store);

    let mdns = Mdns::new().unwrap();

    let behaviour = MyBehaviour {
        kademlia: behaviour,
        mdns: mdns,
        identify: Identify::new(
            String::from("/substrate/1.0"),
            String::from(""),
            local_public_key.clone(),
        ),
    };

    // remote node
    //let remote_peer_id: PeerId = "Qmd6oSuC4tEXHxewrZNdwpVhn8b4NwxpboBCH4kHkH1EYb".parse().unwrap();
    //let remote_addr: Multiaddr = "/ip4/127.0.0.1/tcp/30333".parse().unwrap();
    //ip4/127.0.0.1/tcp/7001/ipfs/12D3KooWEiCzB4bVxyxYwB7xpSvra1Yz1SB39kih4qHpFjDWoC9J
    let remote_peer_id: PeerId = "12D3KooWEiCzB4bVxyxYwB7xpSvra1Yz1SB39kih4qHpFjDWoC9J"
        .parse()
        .unwrap();
    let remote_addr: Multiaddr = "/ip4/127.0.0.1/tcp/07001".parse().unwrap();

    // Create swarm, listen on local port
    let mut swarm = Swarm::new(transport.clone(), behaviour, local_peer_id.clone());
    match Swarm::dial_addr(&mut swarm, remote_addr.clone()) {
        Ok(()) => println!("successfully dialed {:?}", remote_addr),
        Err(e) => println!("dialing {:?} failed with: {:?}", remote_addr, e),
    }

    //make also a transport, behavoir and swarm for ping

    // I think the transport for MyBehaviour doesn't support Ping: error: the trait `libp2p::libp2p_swarm::NetworkBehaviourEventProcess<libp2p::libp2p_ping::PingEvent>` is not implemented for `main::MyBehaviour<libp2p::libp2p_core::muxing::SubstreamRef<std::sync::Arc<libp2p::libp2p_mplex::Multiplex<libp2p::libp2p_core::Negotiated<libp2p::libp2p_secio::SecioOutput<libp2p::libp2p_core::Negotiated<libp2p::libp2p_tcp::TcpTransStream>>>>>>>`

    let ping_transport = libp2p::build_development_transport(local_key.clone());
    let ping_behaviour = Ping::new(PingConfig::new().with_keep_alive(true));
    let mut ping_swarm = Swarm::new(ping_transport, ping_behaviour, local_peer_id.clone());
    match Swarm::dial_addr(&mut ping_swarm, remote_addr.clone()) {
        Ok(()) => println!("successfully dialed {:?} for ping", remote_addr),
        Err(e) => println!("dialing for ping {:?} failed with: {:?}", remote_addr, e),
    };

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();
    Swarm::listen_on(&mut ping_swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

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

        loop {
            match ping_swarm.poll().unwrap() {
                Async::Ready(_) => {}
                Async::NotReady => {
                    if !listening {
                        if let Some(a) = Swarm::listeners(&swarm).next() {
                            info!("Listening locally on {:?}", a);
                            listening = true;
                        }
                    }
                    break;
                }
            }
        }

        for cur_peer in swarm.kademlia.kbuckets_entries() {
            info!("known peer with id {}", cur_peer);
        }

        if node_known {
            swarm.kademlia.bootstrap();
        } else {
            let bucket_size = swarm.kademlia.kbuckets_entries().count();
            info!("number of nodes in the kbuckets {}", bucket_size);
            if swarm.kademlia.kbuckets_entries().count() > 0 {
                node_known = true;
            }
        }

        Ok(Async::NotReady)
    }));
}

use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourAction, PollParameters, ProtocolsHandler, OneShotHandler};
use libp2p::core::{ConnectedPoint, Multiaddr, PeerId, UpgradeInfo, InboundUpgrade, OutboundUpgrade, upgrade};
use tokio::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;

use std::{iter, io};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct ClientConnection<TSubstream> {

    /// Marker to pin the generics.
    marker: PhantomData<TSubstream>,
}

#[derive(Debug, Clone)]
pub enum ClientConnEvent {

}

#[derive(Debug, Clone)]
pub struct ClientConnConfig {

}

impl Default for ClientConnConfig {
    fn default() -> Self {
        ClientConnConfig {

        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientConnRpc {

}

#[derive(Debug, Clone)]
pub enum ClientConnDecodeError {
    Placeholder, // TODO: delete
}

impl From<upgrade::ReadOneError> for ClientConnDecodeError {
    #[inline]
    fn from(_: upgrade::ReadOneError) -> Self {
        ClientConnDecodeError::Placeholder
    }
}

#[derive(Debug, Clone)]
pub enum InnerMessage {
    Placeholder, // TODO: delete
}

impl From<ClientConnRpc> for InnerMessage {
    #[inline]
    fn from(_: ClientConnRpc) -> InnerMessage {
        InnerMessage::Placeholder
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Placeholder
    }
}

impl<TSubstream> NetworkBehaviour for ClientConnection<TSubstream>
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type ProtocolsHandler = OneShotHandler<TSubstream, ClientConnConfig, ClientConnRpc, InnerMessage>;
    type OutEvent = ClientConnEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _: PeerId, _: ConnectedPoint) {

    }

    fn inject_disconnected(&mut self, _: &PeerId, _: ConnectedPoint) {
    }

    fn inject_node_event(&mut self, _: PeerId, _: InnerMessage) {

    }

    fn poll(
        &mut self,
        _: &mut impl PollParameters,
    ) -> Async<
        NetworkBehaviourAction<
            <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    > {
        Async::NotReady
    }

}

impl UpgradeInfo for ClientConnConfig {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    #[inline]
    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/substrate/dot/2")
    }
}

impl UpgradeInfo for ClientConnRpc {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    #[inline]
    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(b"/substrate/dot/2")
    }
}

impl<TSocket> InboundUpgrade<TSocket> for ClientConnConfig
where
    TSocket: AsyncRead + AsyncWrite,
{
    type Output = ClientConnRpc;
    type Error = ClientConnDecodeError;
    type Future = upgrade::ReadOneThen<upgrade::Negotiated<TSocket>, (), fn(Vec<u8>, ()) -> Result<ClientConnRpc, ClientConnDecodeError>>;

    #[inline]
    fn upgrade_inbound(self, socket: upgrade::Negotiated<TSocket>, _: Self::Info) -> Self::Future {
        upgrade::read_one_then(socket, 2048, (), |_packet, ()| {
            Ok(ClientConnRpc {})
        })
    }
}

impl<TSocket> OutboundUpgrade<TSocket> for ClientConnRpc
where
    TSocket: AsyncWrite + AsyncRead,
{
    type Output = ();
    type Error = io::Error;
    type Future = upgrade::WriteOne<upgrade::Negotiated<TSocket>>;

    #[inline]
    fn upgrade_outbound(self, socket: upgrade::Negotiated<TSocket>, _: Self::Info) -> Self::Future {
        let bytes = vec![];
        upgrade::write_one(socket, bytes)
    }
}

impl<TSubstream> Default for ClientConnection<TSubstream> {
    fn default() -> Self {
        ClientConnection {
            marker: PhantomData,
        }
    }
}
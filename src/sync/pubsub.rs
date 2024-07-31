use dashmap::DashMap;
use fastwebsockets::{FragmentCollector, OpCode};
use flume::{Receiver, Sender};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use rkyv::
    AlignedVec
;

use crate::{protocol::messages::{ArchivedProtocolMessage, ProtocolMessage}};
use anyhow::{anyhow, Result};

// pub struct LightweightPacket {
//     pub data: AlignedVec,
// }

// impl LightweightPacket {
//     pub fn to_archive(&self) -> Result<&ArchivedProtocolMessage> {
//         Ok(rkyv::check_archived_root::<ProtocolMessage>(&self.data[..])
//             .map_err(|e| anyhow!("Failed to get archive: {e}"))?)
//     }
// }

// #[derive(Hash, Debug, PartialEq, Eq, Clone)]
// pub struct ChannelDiscriminator((u32, u32));

#[derive(Debug)]
pub struct Kraken
{
    primary: (Sender<AlignedVec>, Receiver<AlignedVec>),
    channels: DashMap<u32, (Sender<AlignedVec>, Receiver<AlignedVec>)>,
    buffer_size: usize,
}

impl Kraken
{
    pub fn new(buffer_size: usize) -> Self {
        Self {
            primary: flume::bounded(buffer_size),
            channels: DashMap::new(),
            buffer_size,
        }
    }
    // pub async fn manage_master(&self, )
    fn optional_create_channel(&self, channel: u32) {
        if !self.channels.contains_key(&channel) {
            let (s, r) = flume::bounded::<AlignedVec>(self.buffer_size);
            self.channels.insert(channel.to_owned(), (s, r));
        }
    }
    pub async fn subscribe(&self, channel: u32) -> Result<AlignedVec> {
        self.optional_create_channel(channel);

        let channel = self.channels.get(&channel).unwrap().1.clone();
        Ok(channel.recv_async().await?)
    }
    pub async fn publish(&self, channel: &u32, data: AlignedVec) -> Result<()> {
        self.optional_create_channel(*channel);

        let channel = self.channels.get(channel).unwrap().0.clone();
        channel.send_async(data).await?;
        Ok(())
    }
    pub async fn recv_primary(&self) -> Result<AlignedVec> {
        Ok(self.primary.1.recv_async().await?)
    }
    pub async fn send_primary(&self, message: ProtocolMessage) -> Result<()> {
        self.primary.0.send_async(message.to_bytes()?).await?;
        Ok(())
    }
}

use std::marker::PhantomData;

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use flume::{Receiver, Sender};
use rkyv::{
    ser::{serializers::AllocSerializer, Serializer},
    validation::validators::DefaultValidator,
    AlignedVec, Archive, CheckBytes, Serialize,
};
use std::hash::Hash;

pub struct TunnelCenter<U, T>
where
    U: Hash + Eq + Copy,
    T: Archive,
{
    primary: (Sender<LightPacket<T>>, Receiver<LightPacket<T>>),
    channels: DashMap<U, (Sender<LightPacket<T>>, Receiver<LightPacket<T>>)>,
    buffer_size: usize,
    _type: PhantomData<T>,
}

impl<'a, U, T> TunnelCenter<U, T>
where
    U: Hash + Eq + Copy,
    T: Archive,
    T: Archive + Serialize<AllocSerializer<0>>,
    T::Archived: CheckBytes<DefaultValidator<'a>>,
{
    pub fn new(buffer: usize) -> Self {
        Self {
            primary: flume::bounded(buffer),
            channels: DashMap::new(),
            buffer_size: buffer,
            _type: PhantomData::default(),
        }
    }
    fn init_channel(&self, channel: U) {
        if !self.channels.contains_key(&channel) {
            self.channels
                .insert(channel, flume::bounded(self.buffer_size));
        }
    }
    pub async fn subscribe(&self, channel: U) -> Result<LightPacket<T>> {
        self.init_channel(channel);

        Ok(self
            .channels
            .get(&channel)
            .unwrap()
            .1
            .clone()
            .recv_async()
            .await?)
    }
    pub async fn publish(&self, channel: U, packet: impl IntoPacket<T>) -> Result<()> {
        self.init_channel(channel);

        self.channels
            .get(&channel)
            .unwrap()
            .0
            .clone()
            .send_async(packet.packetify()?)
            .await.map_err(|e| anyhow!("Failed to publish a message with error: {e}"))?;

        Ok(())
    }
    pub async fn to_tunnel(&self, data: impl IntoPacket<T>) -> Result<()> {
        self.primary
            .0
            .send_async(data.packetify()?)
            .await
            .map_err(|e| anyhow!("Failed sending to tunnel: {e}"))?;
        Ok(())
    }
    pub async fn recv_from_tunnel(&self) -> Result<LightPacket<T>> {
        let d = self
            .primary
            .1
            .recv_async()
            .await
            .map_err(|e| anyhow!("Failed sending to tunnel: {e}"))?;
        Ok(d)
    }
}

pub struct LightPacket<T>
where
    T: Archive,
{
    data: AlignedVec,
    _type: PhantomData<T>,
}

pub trait IntoPacket<T>
where
    T: Archive
{
    fn packetify(self) -> Result<LightPacket<T>>;
}
// impl<'a, T> TryFrom<T> for LightPacket<T>
// where
//     T: Archive + Serialize<AllocSerializer<0>>,
//     T::Archived: CheckBytes<DefaultValidator<'a>>,
// {
//     fn try_from(value: T) -> std::result::Result<Self, Self::Error> {
//         Self::new(value).map_err(|e| Self::Error::)
//     }

// }

impl<'a, T> LightPacket<T>
where
    T: Archive + Serialize<AllocSerializer<0>>,
    T::Archived: CheckBytes<DefaultValidator<'a>>
{
    pub fn new(obj: T) -> Result<LightPacket<T>> {
        let mut serializer = AllocSerializer::<0>::default();
        serializer.serialize_value(&obj)?;
        let bytes = serializer.into_serializer().into_inner();
        Ok(Self {
            data: bytes,
            _type: PhantomData::default(),
        })
    }
    pub fn from_bytes(data: AlignedVec) -> LightPacket<T> {
        Self {
            data,
            _type: PhantomData::default()
        }
    }
    pub fn to_bytes(&self) -> &AlignedVec {
        &self.data
    }
    pub fn access(&'a self) -> Result<&T::Archived> {
        Ok(rkyv::check_archived_root::<T>(&self.data)
            .map_err(|e| anyhow!("Failed to dearchive: {e}"))?)
    }
}

// /// A distributed channel where there is a single point of entry but we can
// /// listen on multiple fronts.
// pub struct FastDistributedSPMC<U, T>
// where
//     U: Hash + Eq + Copy,
// {
//     buffer_size: usize,
//     channel_map: DashMap<U, (Sender<T>, Receiver<T>)>,
// }

// impl<U, T> FastDistributedSPMC<U, T>
// where
//     U: Hash + Eq + Copy,
// {
//     pub fn new(capacity: usize) -> Self {
//         Self {
//             buffer_size: capacity,
//             channel_map: DashMap::<U, (Sender<T>, Receiver<T>)>::new(),
//         }
//     }
//     /// Checks if there is a topic.
//     pub fn has_topic(&self, channel: &U) -> bool {
//         self.channel_map.contains_key(channel)
//     }

//     fn optional_create_channel(&self, channel: &U) {
//         if !self.channel_map.contains_key(&channel) {
//             let (s, r) = flume::bounded::<T>(self.buffer_size);
//             self.channel_map
//                 .insert(*channel, (s, r));
//         }
//     }
//     /// Publishes a message to a channel, creating a channel if it does
//     /// not yet exist.
//     pub async fn publish(&self, channel: &U, message: T) -> Result<()> {
//         self.optional_create_channel(channel);

//         self.channel_map
//             .get(channel)
//             .unwrap().0.clone()
//             .send_async(message)
//             .await
//             .map_err(|e| {
//                 anyhow!("Failed to send message on distributed channel with error {e}.")
//             })?;

//         Ok(())
//     }
//     /// Removes a channel
//     pub fn remove_topic(&self, channel: &U) -> Result<()> {
//         self.channel_map
//             .remove(channel)
//             .ok_or_else(|| anyhow!("Failed to remove distributed channel."))?;
//         Ok(())
//     }
//     /// Subscribes to a channel
//     pub async fn subscribe(&self, channel: &U) -> Result<T> {
//         self.optional_create_channel(channel);

//         Ok(self
//             .channel_map
//             .get(channel)
//             .ok_or_else(|| anyhow!("Found no channel to listen to."))?.1.clone()
//             .recv_async()
//             .await
//             .map_err(|e| anyhow!("Receive failed with {e}."))?)
//     }
// }

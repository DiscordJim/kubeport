use anyhow::{anyhow, Result};
use dashmap::DashMap;
use flume::{Receiver, Sender};
use std::hash::Hash;

/// A distributed channel where there is a single point of entry but we can
/// listen on multiple fronts.
pub struct DistributedSPMC<U, T>
where
    U: Hash + Eq + Clone,
{
    buffer_size: usize,
    channel_map: DashMap<U, (Sender<T>, Receiver<T>)>,
}

impl<U, T> DistributedSPMC<U, T>
where
    U: Hash + Eq + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer_size: capacity,
            channel_map: DashMap::<U, (Sender<T>, Receiver<T>)>::new(),
        }
    }
    /// Checks if there is a topic.
    pub fn has_topic(&self, channel: &U) -> bool {
        self.channel_map.contains_key(channel)
    }

    fn optional_create_channel(&self, channel: &U) {
        if !self.channel_map.contains_key(&channel) {
            let (s, r) = flume::bounded::<T>(self.buffer_size);
            self.channel_map
                .insert(channel.clone(), (s, r));
        }
    } 
    /// Publishes a message to a channel, creating a channel if it does
    /// not yet exist.
    pub async fn publish(&self, channel: &U, message: T) -> Result<()> {
        self.optional_create_channel(channel);

        self.channel_map
            .get(channel)
            .unwrap().0.clone()
            .send_async(message)
            .await
            .map_err(|e| {
                anyhow!("Failed to send message on distributed channel with error {e}.")
            })?;

        Ok(())
    }
    /// Removes a channel
    pub fn remove_topic(&self, channel: &U) -> Result<()> {
        self.channel_map
            .remove(channel)
            .ok_or_else(|| anyhow!("Failed to remove distributed channel."))?;
        Ok(())
    }
    /// Subscribes to a channel
    pub async fn subscribe(&self, channel: &U) -> Result<T> {
        self.optional_create_channel(channel);

        Ok(self
            .channel_map
            .get(channel)
            .ok_or_else(|| anyhow!("Found no channel to listen to."))?.1.clone()
            .recv_async()
            .await
            .map_err(|e| anyhow!("Receive failed with {e}."))?)
    }
}

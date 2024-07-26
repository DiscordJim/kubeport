


use std::{hash::Hash, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use flume::{Receiver, RecvError, SendError, Sender};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use uuid7::Uuid;
use anyhow::anyhow;

pub const CHANNEL_SIZE: usize = 10;



pub struct SimpleChannel<T>{
    channel: (Sender<T>, Receiver<T>),
    pub shutdown: Shutdown
}

pub struct Shutdown(Arc<Notify>, Arc<AtomicBool>);

impl Clone for Shutdown {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0), Arc::clone(&self.1))
    }
}

impl Shutdown {
    pub fn new() -> Self {
        Self(Arc::new(Notify::new()), Arc::new(AtomicBool::new(false)))
    }
    pub fn shutdown(&self) {
        self.1.store(true, Ordering::Release);
        self.0.notify_waiters();
    }
    pub async fn wait_shutdown(&self) {
        self.0.notified().await;
    }
    pub fn is_shutdown(&self) -> bool {
        self.1.load(Ordering::Acquire)
    }
}

impl<T> Clone for SimpleChannel<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            shutdown: self.shutdown.clone()
        }
    }
}
impl<T> SimpleChannel<T> {
    pub fn new(cap: usize) -> Self {
        let channel =flume::bounded(cap);
        Self {
            channel,
            shutdown: Shutdown::new()
        }
    }
    pub async fn send(&self, msg: T) -> Result<()> {
        if self.shutdown.is_shutdown() {
            Err(anyhow!("Channel is already shutdown."))?
        }
        Ok(tokio::select! {
            m = self.channel.0.send_async(msg) => {
                println!("We did send message");
                m
            },
            _ = self.shutdown.wait_shutdown() => {
                Err(anyhow!("Channel shutdown."))?
            }
        }.map_err(|e| anyhow!("Send failure: {e}"))?)
        
       // self.channel.0.send_async(msg).await
    }
    pub async fn recv(&self) -> Result<T> {
        if self.shutdown.is_shutdown() {
            Err(anyhow!("Channel is already shutdown."))?
        }
        Ok(tokio::select! {
            m = self.channel.1.recv_async() => m,
            _ = self.shutdown.wait_shutdown() => {
                Err(anyhow!("Channel shutdown."))?
            }
        }.map_err(|e| anyhow!("Send failure: {e}"))?)
        
        // self.channel.1.recv_async().await
    }
}

pub struct ChannelDistributor<U, T>
    where U: Hash + Eq + Clone
    {
    buf_size: usize,
    cmap: DashMap<U, Arc<SimpleChannel<T>>>
}

impl<U, T> ChannelDistributor<U, T>
    where U: Hash + Eq + Clone
    {
    pub fn new(cap: usize) -> Self {
        Self {
            buf_size: cap,
            cmap: DashMap::<U, Arc<SimpleChannel<T>>>::new()
        }
    }
    pub async fn get_channel(&self, id: &U) -> Arc<SimpleChannel<T>> {
        if !self.cmap.contains_key(&id) {
            self.cmap.insert(id.clone(), Arc::new(SimpleChannel::new(self.buf_size)));
        }
        Arc::clone(self.cmap.get(&id).unwrap().value())
    }
    pub fn has_channel(&self, id: &U) -> bool {
        self.cmap.contains_key(id)
    }
    pub fn remove_channel(&self, id: &U) {
        self.cmap.remove(id);
    }
    // pub async fn submit(&self, id: &U, msg: T) -> Result<()> {
    //     if !self.cmap.contains_key(&id) {
    //         self.cmap.insert(id.clone(), SimpleChannel::new(self.buf_size));
    //     }
    //     self.cmap.get(&id).ok_or_else(|| anyhow!("No channel found."))?.send(msg).await.map_err(|_| anyhow!("Failed to send."))?;
    //     Ok(())
    // }
    // pub async fn recv(&self, id: &U) -> Result<T> {
    //     if !self.cmap.contains_key(&id) {
    //         self.cmap.insert(id.clone(), SimpleChannel::new(self.buf_size));
    //     }
    //     Ok(self.cmap.get(id).ok_or_else(|| anyhow!("No channel found."))?.recv().await?)
    // }
}


pub struct WebsocketProxy {
    communicator: SimpleChannel<ProtocolMessage>,
    distributor: Arc<ChannelDistributor<Uuid, WebsocketMessage>>
    //recv: Receiver<WebsocketMessage>
}

impl Clone for WebsocketProxy {
    fn clone(&self) -> Self {
        Self {
            communicator: self.communicator.clone(),
            distributor: Arc::clone(&self.distributor)
        }
    }
}



#[derive(Serialize, Deserialize, Debug)]
pub struct WebsocketMessage {
    pub id: Uuid,
    pub data: Vec<u8>
}




#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolMessage {
    Open(Uuid),
    Message(WebsocketMessage)

}

impl ProtocolMessage {
    pub async fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
}

impl WebsocketProxy {
    pub fn create(ws: WebSocket) -> Self {
        let wschannel = SimpleChannel::<ProtocolMessage>::new(CHANNEL_SIZE);
        // let (send, recv) = flume::bounded::<ProtocolMessage>(300);


        let distributor = Arc::new(ChannelDistributor::new(CHANNEL_SIZE));

        let (mut ws_sender, mut ws_receiver) = ws.split();

        tokio::spawn({
            //let send = send.clone();
            let distributor = Arc::clone(&distributor);
            async move {
                while let Some(Ok(Message::Binary(bin))) = ws_receiver.next().await {
                    if let Ok(ProtocolMessage::Message(msg)) = bincode::deserialize(&bin) {
                        let id = msg.id.clone();
                        if !distributor.has_channel(&id) {
                            continue
                        }
                        println!("Submitting to distributor... {} {}", msg.id, msg.data.len());
                       
                        if let Err(e) = distributor.get_channel(&id).await.send(msg).await {
                            println!("Shutting down distributor..");
                            distributor.remove_channel(&id);
                        }
                        // match distributor.get_channel(&msg.id).await.recv().await {
                        //     Ok(v) => 
                        // }
                        // distributor.submit(&msg.id.clone(), msg).await.unwrap();
                    } 
                }
            }
        });

        tokio::spawn({
            // let recv = recv.clone();
            let wschannel = wschannel.clone();
            async move {
                while let Ok(msg) = wschannel.recv().await {
                    println!("Got a message");
                    ws_sender.send(Message::Binary(msg.serialize().await.unwrap())).await.unwrap();
                }
            }
        });




        Self {
            communicator: wschannel,
            distributor
        }
    }

    // pub async fn recv(&self, id: &Uuid) -> Result<WebsocketMessage> {
    //     Ok(self.distributor.recv(id).await?)
    // }
    pub async fn send_main(&self, msg: ProtocolMessage) -> Result<()> {
        self.communicator.send(msg).await?;
        Ok(())
    }
    pub async fn get_channel(&self, id: &Uuid) -> Arc<SimpleChannel<WebsocketMessage>> {
        self.distributor.get_channel(id).await
    }
    // pub async fn send(&self, id: &Uuid, data: Vec<u8>) -> Result<()> {
    //     // println!("|\t-> Data: {}", data.len());
    //     self.communicator.send(ProtocolMessage::Message(WebsocketMessage {
    //         id: id.clone(),
    //         data
    //     })).await?;

    //     Ok(())
    // }
}



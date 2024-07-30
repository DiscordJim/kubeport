


use std::{hash::Hash, path::Path, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use flume::{Receiver, RecvError, SendError, Sender};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tracing::{error, info, Level};
use tracing_appender::rolling::daily;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, FmtSubscriber};
use uuid7::Uuid;
use anyhow::anyhow;

use crate::sync::{coordinator::AsyncCoordinator, distributed::DistributedSPMC};

pub const CHANNEL_SIZE: usize = 10;


pub fn configure_system_logger(path: impl AsRef<Path>) {
    let file_appender =
        daily(path.as_ref().join("logs"), "prefix.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let format = tracing_subscriber::fmt::format().compact();

    FmtSubscriber::builder()
        .event_format(format)
        .with_max_level(Level::INFO)
        .finish()
        .with(
            Layer::default()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    Box::leak(Box::new(guard));
}


pub struct SimpleChannel<T>{
    channel: (Sender<T>, Receiver<T>),
    //pub shutdown: Arc<AsyncCoordinator>
}



impl<T> Clone for SimpleChannel<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            //shutdown: self.shutdown.clone()
        }
    }
}
impl<T> SimpleChannel<T> {
    pub fn new(cap: usize) -> Self {
        let channel =flume::bounded(cap);
        Self {
            channel,
            //shutdown: Arc::new(AsyncCoordinator::new())
        }
    }
    pub async fn send(&self, msg: T) -> Result<()> {
        // if self.shutdown.is_shutdown() {
        //     Err(anyhow!("Channel is already shutdown."))?
        // }
        Ok(tokio::select! {
            m = self.channel.0.send_async(msg) => {
                println!("We did send message");
                m
            },
            // _ = self.shutdown.wait_on_change() => {
            //     Err(anyhow!("Channel shutdown."))?
            // }
        }.map_err(|e| anyhow!("Send failure: {e}"))?)
        
       // self.channel.0.send_async(msg).await
    }
    pub async fn recv(&self) -> Result<T> {
        // if self.shutdown.is_shutdown() {
        //     Err(anyhow!("Channel is already shutdown."))?
        // }
        Ok(tokio::select! {
            m = self.channel.1.recv_async() => m,
            // _ = self.shutdown.wait_on_change() => {
            //     Err(anyhow!("Channel shutdown."))?
            // }
        }.map_err(|e| anyhow!("Send failure: {e}"))?)
        
        // self.channel.1.recv_async().await
    }
}

pub struct ChannelDistributor<U, T>
    where
        U: Hash + Eq + Clone
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
    pub fn has_topic(&self, id: &U) -> bool {
        self.cmap.contains_key(id)
    }
    pub fn remove_topic(&self, id: &U) {
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
    coordinator: Arc<AsyncCoordinator>,
    //distributor: Arc<ChannelDistributor<Uuid, WebsocketMessage>>
    distributor: Arc<DistributedSPMC<Uuid, WebsocketMessage>>
    //recv: Receiver<WebsocketMessage>
}

impl Clone for WebsocketProxy {
    fn clone(&self) -> Self {
        Self {
            coordinator: Arc::clone(&self.coordinator),
            communicator: self.communicator.clone(),
            distributor: Arc::clone(&self.distributor)
        }
    }
}


#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum ControlCode {
    Neutral,
    Open,
    Close
}


#[derive(Serialize, Deserialize, Debug)]
pub struct WebsocketMessage {
    pub id: Uuid,
    pub code: ControlCode,
    pub data: Vec<u8>
}




#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolMessage {
    //Open(Uuid),
    Message(WebsocketMessage),
    // Close(Uuid)

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

        let coordinator = Arc::new(AsyncCoordinator::new());


        //let distributor = Arc::new(ChannelDistributor::new(CHANNEL_SIZE));
        let distributor = Arc::new(DistributedSPMC::new(CHANNEL_SIZE));

        let (mut ws_sender, mut ws_receiver) = ws.split();

        tokio::spawn({
            let coordinator = Arc::clone(&coordinator);
            //let send = send.clone();
            println!("Creating listener.");
            let distributor = Arc::clone(&distributor);
            async move {

                loop {
                    match tokio::select! {
                        p = ws_receiver.next() => p,
                        _ = coordinator.wait_on_change() => {
                            //error!("Detected a state change.");
                            break
                        }
                    } {
                        Some(Ok(message)) => {
                            if let Message::Binary(content) = message {
                                println!("Getting raw message");
                                if let Ok(ProtocolMessage::Message(msg)) = bincode::deserialize(&content) {
                                    let id = msg.id.clone();
                                    println!("Getting message.");
                                    if !distributor.has_topic(&id) {
                                        continue
                                    }
                                    println!("Submitting to distributor... {} {}", msg.id, msg.data.len());
                                
                                    //if let Err(e) = distributor.get_channel(&id).await.send(msg).await {
                                    if let Err(e) = distributor.publish(&id, msg).await { 
                                        error!("Failed to publish to distributor with error: {e}. Doing best effort to remove the topic.");
                                        let _ = distributor.remove_topic(&id);
                                    }
                                    // match distributor.get_channel(&msg.id).await.recv().await {
                                    //     Ok(v) => 
                                    // }
                                    // distributor.submit(&msg.id.clone(), msg).await.unwrap();
                                } 
                            }
    
                        },
                        Some(Err(e)) => {
                            error!("Write loop received an error: {e}. Shutting down.");
                            coordinator.shutdown();
                            break
                        },
                        None => {
                            error!("Write loop received none as a message. Shutting down.");
                            coordinator.shutdown();
                            break
                        }
                    }
                }

                


                // while let Message::Binary(bin) = ws_receiver.next().await.unwrap().unwrap() {
                    
                // }
                info!("Write loop succesfully shutdown.");
            }
            
        });

        tokio::spawn({
            // let recv = recv.clone();
            let coordinator = Arc::clone(&coordinator);
            let wschannel = wschannel.clone();
            async move {
                loop {

                    //println!("Waiting");
                    match tokio::select! {
                        p = wschannel.recv() => p,
                        _ = coordinator.wait_on_change() => {
                            //error!("Detected a state change.");
                            break
                        }
                    } {
                        Ok(protocol_message) => {
                            ws_sender.send(Message::Binary(protocol_message.serialize().await.unwrap())).await.unwrap();
                        },
                        Err(e) => {
                            error!("The receiving channel encountered an error: {e}");
                        }
                    }
                }

                info!("Read loop succesfully shutdown.");
                

                // while let Ok(msg) = wschannel.recv().await {
                //     println!("Got a message");
                //     ws_sender.send(Message::Binary(msg.serialize().await.unwrap())).await.unwrap();
                // }
                // info!("Shut down stream.");
            }
        
        });




        Self {
            communicator: wschannel,
            coordinator,
            distributor
        }
    }
    pub async fn send_main(&self, msg: ProtocolMessage) -> Result<()> {
        self.communicator.send(msg).await?;
        Ok(())
    }
    // pub async fn get_channel(&self, id: &Uuid) -> Arc<SimpleChannel<WebsocketMessage>> {
    //     self.distributor.get_channel(id).await
    // }
    pub fn get_distributor(&self) -> Arc<DistributedSPMC<Uuid, WebsocketMessage>> {
        Arc::clone(&self.distributor)
    }
    pub fn get_coordinator(&self) -> Arc<AsyncCoordinator> {
        Arc::clone(&self.coordinator)
    }
}





use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{tcp::OwnedWriteHalf, TcpStream}, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid7::Uuid;

use crate::commons::{ProtocolMessage, WebsocketMessage};







pub async fn run_client() -> Result<()> {

    let service_port: u16 = 4032;


    let ws_stream = connect_async("ws://localhost:8000/forward/name").await?.0;
    let (ws_write, mut ws_read) = ws_stream.split();
    
    let ws_write = Arc::new(Mutex::new(ws_write));

    let connection_map = Arc::new(DashMap::<Uuid, OwnedWriteHalf>::new());

    

    loop {
        if let Some(Ok(Message::Binary(msg))) = ws_read.next().await {
            match bincode::deserialize::<ProtocolMessage>(&msg)? {
                ProtocolMessage::Message(msg) => {
                    if !connection_map.contains_key(&msg.id) {
                        continue;
                    }
                    if let Some(mut res) = connection_map.get_mut(&msg.id) {
                        println!("| -> Write {}", msg.data.len());
                        res.write_all(&msg.data).await?;
                    }
                },
                ProtocolMessage::Open(msg) => {
                    println!("Opening a new channel under {}.", msg);
                    if let Ok(stream) = TcpStream::connect(SocketAddr::from(([127,0,0,1], service_port))).await {
                        // let guarded = Arc::new(stream);
                        println!("Connected to service succesfully.");
    
                        let (mut read, write) = stream.into_split();
    
                        tokio::spawn({
                            let id = msg.clone();
                            let ws_write = Arc::clone(&ws_write);
                            let connection_map = Arc::clone(&connection_map);
                            async move {
                                loop {
                                    let mut buffer = [0u8; 16409];
                                    println!("Reading...");
                                    if let Ok(bytes_read) = read.read(&mut buffer).await {
                                        println!("Got paccket.. {}", bytes_read);
                                        if bytes_read == 0 {
                                            connection_map.remove(&id);
                                            break;
                                        }
                                        if let Ok(pbytes) = ProtocolMessage::Message(WebsocketMessage {
                                            id,
                                            data: buffer[..bytes_read].to_vec()
                                        }).serialize().await {
                                            println!("| -> Writing {}", pbytes.len());
                                            ws_write.lock().await.send(Message::Binary(pbytes)).await.unwrap();
                                        }
                                        
                                    }
                                }
                        }});
    
    
                        connection_map.insert(msg, write);
                    }
                }
            }
        }
    
    }

    // println!("Connected to the websocket.");

    // let conn = Box::leak(Box::new(TcpStream::connect("127.0.0.1:4032").await?));
    
    // println!("Fully connected.");


    // let (mut local_read, mut local_write) = conn.split();
    // let (mut remote_write, mut remote_read) = ws_stream.split();



    // tokio::spawn(async move {
    //     loop {
    //         let mut buffer = [0u8; 1024];
    //         let b = local_read.read(&mut buffer).await.unwrap();
    //         //println!("Received some bytes. Forwarding them.");
    //         remote_write.send(Message::Binary(buffer[..b].to_vec())).await.unwrap();
    //     }
    // });

    // loop {
    //     if let Some(Ok(Message::Binary(mut wow))) = remote_read.next().await{
    //         local_write.write_all(&mut wow).await.unwrap();
    //     }
    // }

    
}
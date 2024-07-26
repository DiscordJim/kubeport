

use std::{net::SocketAddr, process::exit, ptr::read, str::from_utf8, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{tcp::OwnedWriteHalf, TcpStream}, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid7::Uuid;

use crate::commons::{ControlCode, ProtocolMessage, WebsocketMessage};







pub async fn run_client() -> Result<()> {

    let service_port: u16 = 4032;


    // if let Ok(mut stream) = TcpStream::connect("localhost:4032").await {
    //     println!("Connecting to service.");


    //     stream.write_all(&tokio::fs::read("request.txt").await.unwrap()).await.unwrap();
    //     stream.write_all(b"\r\n").await.unwrap();
    //     println!("WROTE OUT BYTES");

    //     let buf: &mut [u8] = &mut [0u8; 8096];
    //     while let Ok(data) = stream.read(buf).await {
    //         if data == 0 {
    //             println!("Done transmission.");
    //             break;
    //         }

    //         println!("GOT SOME DATA");
    //         // if let Ok(pbytes) = ProtocolMessage::Message(WebsocketMessage {
    //         //     id: msg.id.clone(),
    //         //     data: buf[..data].to_vec()
    //         // }).serialize().await {
    //         //     println!("| -> Push {} bytes", pbytes.len());
    //         //     ws_write.lock().await.send(Message::Binary(pbytes)).await.unwrap();
    //         // }
    //     }


    // }


    // exit(1);


    let ws_stream = connect_async("ws://localhost:8000/forward/name").await?.0;
    let (ws_write, mut ws_read) = ws_stream.split();
    
    let ws_write = Arc::new(Mutex::new(ws_write));

    let connection_map = Arc::new(DashMap::<Uuid, OwnedWriteHalf>::new());

    

    loop {
        if let Some(Ok(Message::Binary(msg))) = ws_read.next().await {
            match bincode::deserialize::<ProtocolMessage>(&msg)? {
                ProtocolMessage::Message(msg) => {

                    if let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], service_port))).await {
                        println!("Connecting to service.");


                        stream.write_all(&msg.data).await.unwrap();
                        println!("SENT BYTES.");

                        let buf: &mut [u8] = &mut [0u8; 8096];
                        while let Ok(data) = stream.read(buf).await {
                            if data == 0 {
                                println!("Done transmission.");

                                let protocol = ProtocolMessage::Message(WebsocketMessage {
                                    id: msg.id.clone(),
                                    code: ControlCode::Close,
                                    data: Vec::new()
                                });

                                ws_write.lock().await.send(Message::Binary(protocol.serialize().await.unwrap())).await.unwrap();
                                println!("send");
                                break;
                            }
                            if let Ok(pbytes) = ProtocolMessage::Message(WebsocketMessage {
                                id: msg.id.clone(),
                                code: ControlCode::Neutral,
                                data: buf[..data].to_vec()
                            }).serialize().await {
                                println!("| -> Push {} bytes", pbytes.len());
                                println!("| MESSAGE: {:?}", from_utf8(&buf[..data]));
                                ws_write.lock().await.send(Message::Binary(pbytes)).await.unwrap();
                            }
                        }
                        println!("Bro");
                    }

                    // if !connection_map.contains_key(&msg.id) {
                    //     continue;
                    // }
                    // if let Some(mut res) = connection_map.get_mut(&msg.id) {
                    //     println!("| <- Pull {} bytes", msg.data.len());
                    //     println!("| MESSAGE: {:?}", from_utf8(&msg.data));
                    //     res.write_all(&msg.data).await?;
                    // }
                },
                _ => {}
                // ProtocolMessage::Open(msg) => {
                //     // println!("Opening a new channel under {}.", msg);
                //     // if let Ok(stream) = TcpStream::connect(SocketAddr::from(([127,0,0,1], service_port))).await {
                //     //     // let guarded = Arc::new(stream);
                //     //     println!("Connected to service succesfully.");
    
                //     //     let (mut read, write) = stream.into_split();
                //     //     connection_map.insert(msg, write);


                //     //     tokio::spawn({
                //     //         let id = msg.clone();
                //     //         let ws_write = Arc::clone(&ws_write);
                //     //         let connection_map = Arc::clone(&connection_map);
                //     //         async move {
                //     //             loop {
                //     //                 let mut buffer = [0u8; 16409];
                //     //                 println!("Reading...");
                //     //                 if let Ok(bytes_read) = read.read(&mut buffer).await {
                //     //                     println!("Got paccket.. {}", bytes_read);
                //     //                     if bytes_read == 0 {
                //     //                         connection_map.remove(&id);
                //     //                         break;
                //     //                     }
                //     //                     if let Ok(pbytes) = ProtocolMessage::Message(WebsocketMessage {
                //     //                         id,
                //     //                         data: buffer[..bytes_read].to_vec()
                //     //                     }).serialize().await {
                //     //                         println!("| -> Push {} bytes", pbytes.len());
                //     //                         ws_write.lock().await.send(Message::Binary(pbytes)).await.unwrap();
                //     //                     }
                                        
                //     //                 }
                //     //             }
                //     //     }});
    
    
                       
                //     }
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

    

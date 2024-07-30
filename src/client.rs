

use std::{net::SocketAddr, str::from_utf8};

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{tcp::OwnedWriteHalf, TcpStream}, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};

use crate::commons::{configure_system_logger, ControlCode, ProtocolMessage, WebsocketMessage};





const SERVICE_NAME: &str = "name";
const LOCAL_SERVICE_PORT: u16 = 4032;

pub async fn handle_message(ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, bytes: Vec<u8>) -> Result<()> {

    match bincode::deserialize::<ProtocolMessage>(&bytes)? {
        ProtocolMessage::Message(msg) => {

            if let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], LOCAL_SERVICE_PORT))).await {
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

                        ws_stream.send(Message::Binary(protocol.serialize().await.unwrap())).await.unwrap();
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
                        ws_stream.send(Message::Binary(pbytes)).await.unwrap();
                    }
                }
                println!("Bro");
            }
        }
    }
            
    Ok(())
}



pub async fn run_client() -> Result<()> {


    configure_system_logger("logs");


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


    info!("Starting client service...");
    let ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>> = &mut connect_async(format!("ws://localhost:8000/forward/{SERVICE_NAME}")).await?.0;
    info!("Forwarding 127.0.0.1:{LOCAL_SERVICE_PORT} -> remote::/[{SERVICE_NAME}]");

    //let (ws_write, mut ws_read) = ws_stream.split();
    
    //let ws_write = Arc::new(Mutex::new(ws_write));

   // let connection_map = Arc::new(DashMap::<Uuid, OwnedWriteHalf>::new());

    

    loop {
//ws_stream.is_terminated()
        
        match ws_stream.next().await {
            Some(Ok(v)) => {
                if let Message::Binary(bytes) = v {
                    handle_message(ws_stream, bytes).await?;
                }
               
            },
            Some(Err(e)) => {
                error!("Failed to receive a message from the WebSocket with error: {e}");
            },
            None => {
                error!("Received a null packet from the remote server. This means the connection has closed.");
                break;
            }
        }


        

                    // if !connection_map.contains_key(&msg.id) {
                    //     continue;
                    // }
                    // if let Some(mut res) = connection_map.get_mut(&msg.id) {
                    //     println!("| <- Pull {} bytes", msg.data.len());
                    //     println!("| MESSAGE: {:?}", from_utf8(&msg.data));
                    //     res.write_all(&msg.data).await?;
                    // }
                // }
        //         // ProtocolMessage::Open(msg) => {
        //         //     // println!("Opening a new channel under {}.", msg);
        //         //     // if let Ok(stream) = TcpStream::connect(SocketAddr::from(([127,0,0,1], service_port))).await {
        //         //     //     // let guarded = Arc::new(stream);
        //         //     //     println!("Connected to service succesfully.");
    
        //         //     //     let (mut read, write) = stream.into_split();
        //         //     //     connection_map.insert(msg, write);


        //         //     //     tokio::spawn({
        //         //     //         let id = msg.clone();
        //         //     //         let ws_write = Arc::clone(&ws_write);
        //         //     //         let connection_map = Arc::clone(&connection_map);
        //         //     //         async move {
        //         //     //             loop {
        //         //     //                 let mut buffer = [0u8; 16409];
        //         //     //                 println!("Reading...");
        //         //     //                 if let Ok(bytes_read) = read.read(&mut buffer).await {
        //         //     //                     println!("Got paccket.. {}", bytes_read);
        //         //     //                     if bytes_read == 0 {
        //         //     //                         connection_map.remove(&id);
        //         //     //                         break;
        //         //     //                     }
        //         //     //                     if let Ok(pbytes) = ProtocolMessage::Message(WebsocketMessage {
        //         //     //                         id,
        //         //     //                         data: buffer[..bytes_read].to_vec()
        //         //     //                     }).serialize().await {
        //         //     //                         println!("| -> Push {} bytes", pbytes.len());
        //         //     //                         ws_write.lock().await.send(Message::Binary(pbytes)).await.unwrap();
        //         //     //                     }
                                        
        //         //     //                 }
        //         //     //             }
        //         //     //     }});
    
    
                       
        //         //     }
        //         }
        //     }
        }

        Ok(())
    
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

    

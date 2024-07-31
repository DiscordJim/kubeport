

use std::{net::SocketAddr, process::exit, str::from_utf8};

use anyhow::Result;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use rkyv::{ser::{serializers::AllocSerializer, Serializer}, AlignedVec, Archive, Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{tcp::OwnedWriteHalf, TcpStream}, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};
use uuid7::Uuid;

use crate::{commons::configure_system_logger, protocol::messages::{ArchivedProtocolMessage, ControlCode, ProtocolMessage, WebsocketMessage}};


#[derive(Debug)]
pub struct ConnectionState {
    stream: TcpStream
}

static CONN_MAP: OnceCell<DashMap<u32, ConnectionState>> = OnceCell::new();




const SERVICE_NAME: &str = "name";
const LOCAL_SERVICE_PORT: u16 = 4032;

pub async fn handle_message(ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>, bytes: Vec<u8>) -> Result<()> {
    

    let mut nv = AlignedVec::new();
    nv.extend_from_slice(&bytes);

    let bytes = nv;


    match ProtocolMessage::from_bytes(&bytes)? {
        ArchivedProtocolMessage::Open(id) => {
            info!("Received request to open up.");
            if let Ok(stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], LOCAL_SERVICE_PORT))).await {
                info!("Succesfully opened a new stream.");
                CONN_MAP.get().unwrap().insert(*id, ConnectionState {
                    stream
                });
            }
            
        },
        ArchivedProtocolMessage::Message(msg) => {

            // if let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], LOCAL_SERVICE_PORT))).await {
            //     println!("Connecting to service.");


            let stream = &mut CONN_MAP.get().unwrap().get_mut(&msg.id).unwrap().stream;

                stream.write_all(&msg.data).await.unwrap();
                println!("SENT BYTES.");

                let buf: &mut [u8] = &mut [0u8; 8096];
                while let Ok(data) = stream.read(buf).await {
                    if data == 0 {
                        info!("Done transmission.");

                        let protocol = ProtocolMessage::Message(WebsocketMessage {
                            id: msg.id.clone(),
                            code: ControlCode::Close,
                            data: Vec::new()
                        });

                        ws_stream.send(Message::Binary(protocol.to_bytes().unwrap().into_vec())).await.unwrap();
                        println!("send");
                        break;
                    }
                    if let Ok(pbytes) = ProtocolMessage::Message(WebsocketMessage {
                        id: msg.id.clone(),
                        code: ControlCode::Neutral,
                        data: buf[..data].to_vec()
                    }).to_bytes() {
                        println!("| -> Push {} bytes", pbytes.len());
                       // println!("| MESSAGE: {:?}", from_utf8(&buf[..data]));
                        ws_stream.send(Message::Binary(pbytes.into_vec())).await.unwrap();
                    }
                }
                println!("Bro");
            // }
        },
        _ => {}
    }
            
    Ok(())
}


use anyhow::Error;

#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
pub struct Test {
    int: u8
}


pub async fn run_client() -> Result<()> {

    // let test = Test { 
    //     int: 3
    // };

    // let mut serializer = AllocSerializer::<0>::default();
    // serializer.serialize_value(&test).unwrap();
    // let bytes = serializer.into_serializer().into_inner();

    // // let bytes = rkyv::to_bytes::<_, 4>(&test).unwrap();

    // let archived = rkyv::access::<Test>(&bytes[..]).unwrap();
    // println!("Archived: {:?}", test.int);


    // let proc = ProtocolMessage::Establish(1).to_bytes()?;
    // println!("proc: {:?}", proc);

    // let proc = ProtocolMessage::from_bytes(&proc[..])?;
    // println!("DESERIALIZED");

    // exit(1);


    configure_system_logger("logs");




    CONN_MAP.set(DashMap::new()).unwrap();


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



    info!("Sending establishment...");
    let liquid = ProtocolMessage::Establish(0).to_bytes()?;
    // println!("Bytes: {:?}", liquid);

    // // let mut bro = AlignedVec::new();
    // // bro.extend_from_slice(liquid.as_ref());

    // // ProtocolMessage::from_bytes(bro.as_ref()).unwrap();
    // println!("Deserialized {:?}", bro.len());
    ws_stream.send(Message::Binary(liquid.into_vec())).await?;

    // ws_stream.send(ProtocolMessage::Establish(0).to_message().await.unwrap()).await.unwrap();

    info!("Establishment sent...");
    //let (ws_write, mut ws_read) = ws_stream.split();
    
    //let ws_write = Arc::new(Mutex::new(ws_write));

   // let connection_map = Arc::new(DashMap::<Uuid, OwnedWriteHalf>::new());

    

    loop {
//ws_stream.is_terminated()
        
        match ws_stream.next().await {
            Some(Ok(v)) => {
                if let Message::Binary(bytes) = v {
                    println!("Received a message.");
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

    

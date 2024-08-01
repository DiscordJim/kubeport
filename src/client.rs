

use std::{net::SocketAddr, process::exit, str::from_utf8, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use rkyv::{ser::{serializers::AllocSerializer, Serializer}, AlignedVec, Archive, Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream}, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};
use uuid7::Uuid;

use crate::{commons::configure_system_logger, protocol::{messages::{ArchivedProtocolMessage, ControlCode, ProtocolMessage, WebsocketMessage}, stream::{SequencedRead, SequencedStream, SequencedWrite}}};







const SERVICE_NAME: &str = "name";
const LOCAL_SERVICE_PORT: u16 = 4032;

const BUFFER_SIZE: usize = 4096;


pub async fn handle_socket_reading(state: Arc<ClientState>, id: u32) -> Result<()> {

    loop {
        println!("Reading socket");
        let buf = &mut [0u8; BUFFER_SIZE];
        let bytes = state.read_map.get_mut(&id).unwrap().read(buf).await?;

        println!("Receiving {}", bytes);
        if bytes == 0 {
            println!("TCP connection closed.");
            break;
        }
        state.write_end.lock().await.send(ProtocolMessage::Message(WebsocketMessage {
            id,
            code: ControlCode::Neutral,
            data: Vec::from(&buf[..bytes])
        }).to_bytes()?).await?;
        


    }
    Ok(())
}

pub async fn start_new_connection(state: &Arc<ClientState>, id: u32) -> Result<()> {
    info!("Received request to open up.");
    if let Ok(stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], LOCAL_SERVICE_PORT))).await {
        info!("Succesfully opened a new stream.");
        let (read, write) = stream.into_split();
        state.write_map.insert(id, write);
        state.read_map.insert(id, read);

        tokio::spawn(handle_socket_reading(Arc::clone(state), id));
        // CONN_MAP.get().unwrap().insert(*id, ConnectionState {
        //     stream
        // });
    }
    Ok(())
} 



pub async fn handle_message(state: &Arc<ClientState>,  bytes: AlignedVec) -> Result<()> {
    

    // let mut nv = AlignedVec::new();
    // nv.extend_from_slice(&bytes);

    // let bytes = nv;


    match ProtocolMessage::from_bytes(&bytes)? {
        ArchivedProtocolMessage::Open(id) => start_new_connection(&state, *id).await?,
        ArchivedProtocolMessage::Message(msg) => {
            state.send(msg.id, &msg.data).await?;


          


            // let stream = &mut CONN_MAP.get().unwrap().get_mut(&msg.id).unwrap().stream;

            //     stream.write_all(&msg.data).await.unwrap();
            //     println!("SENT BYTES.");

            //     let buf: &mut [u8] = &mut [0u8; 8096];
            //     while let Ok(data) = stream.read(buf).await {
            //         if data == 0 {
            //             info!("Done transmission.");

            //             let protocol = ProtocolMessage::Message(WebsocketMessage {
            //                 id: msg.id.clone(),
            //                 code: ControlCode::Close,
            //                 data: Vec::new()
            //             });

            //             ws_stream.send(protocol.to_bytes()?).await?;
            //             //ws_stream.send(Message::Binary(protocol.to_bytes().unwrap().into_vec())).await.unwrap();
            //             println!("send");
            //             break;
            //         }
            //         if let Ok(pbytes) = ProtocolMessage::Message(WebsocketMessage {
            //             id: msg.id.clone(),
            //             code: ControlCode::Neutral,
            //             data: buf[..data].to_vec()
            //         }).to_bytes() {
            //             println!("| -> Push {} bytes", pbytes.len());
            //            println!("| MESSAGE: {:?}", from_utf8(&buf[..data]));
            //            ws_stream.send(pbytes).await?;
            //             // ws_stream.send(Message::Binary(pbytes.into_vec())).await.unwrap();
            //         }
            //     }
            
            // }
        },
        _ => {}
    }
            
    Ok(())
}


use anyhow::Error;


pub struct ClientState {
    write_map: DashMap<u32, OwnedWriteHalf>,
    read_map: DashMap<u32, OwnedReadHalf>,
    write_end: Mutex<SequencedWrite>,
}


impl ClientState {
    pub async fn send(&self, id: u32, bytes: &[u8]) -> Result<()> {
        self.write_map.get_mut(&id).unwrap().write_all(bytes).await?;
        Ok(())
    }
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




    // CONN_MAP.set(DashMap::new()).unwrap();


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
    let mut ws_stream = SequencedStream::new(TcpStream::connect("127.0.0.1:8000").await?);
    info!("Forwarding 127.0.0.1:{LOCAL_SERVICE_PORT} -> remote::/[{SERVICE_NAME}]");


    


    info!("Establishing connection...");
    ws_stream.send(ProtocolMessage::Establish(0).to_bytes()?).await?;
    info!("Connection established.");
    

    let (mut read_end, write_end) = ws_stream.into_split();


    let state = Arc::new(ClientState {
        write_map: DashMap::new(),
        read_map: DashMap::new(),
        write_end: Mutex::new(write_end)
    });




    loop {
        let packet = read_end.recv().await?;
        handle_message(&state ,packet).await?;

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

    

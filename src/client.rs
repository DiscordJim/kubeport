

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use dashmap::DashMap;
use futures_util::StreamExt;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid7::Uuid;

use crate::commons::ProtocolMessage;







pub async fn run_client() -> Result<()> {

    let service_port: u16 = 4032;


    let mut ws_stream = connect_async("ws://localhost:8000/forward/name").await?.0;
    let connection_map: DashMap<Uuid, Arc<Mutex<TcpStream>>> = DashMap::new();

    

    'master: while let Some(Ok(Message::Binary(msg))) = ws_stream.next().await {
        match bincode::deserialize::<ProtocolMessage>(&msg)? {
            ProtocolMessage::Message(msg) => {
                if !connection_map.contains_key(&msg.id) {
                    continue 'master;
                }
                let b = connection_map.get(&msg.id);

            },
            ProtocolMessage::Open(msg) => {
                println!("Opening a new channel under {}.", msg);
                if let Ok(stream) = TcpStream::connect(SocketAddr::from(([127,0,0,1], service_port))).await {
                    let guarded = Arc::new(Mutex::new(stream));



                    connection_map.insert(msg, Arc::clone(&guarded));
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


    Ok(())
    
}
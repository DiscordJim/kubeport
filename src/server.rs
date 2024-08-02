

use std::{borrow::BorrowMut, collections::{HashMap, HashSet}, net::SocketAddr, ops::Index, process::exit, str::from_utf8, sync::Arc, usize};

use anyhow::Result;

use dashmap::DashMap;


use rand::Rng;
use rkyv::AlignedVec;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream, UdpSocket}, sync::{Notify, OnceCell}};
use tracing::info;

use fastwebsockets::Frame;

use crate::{commons::{configure_system_logger, CHANNEL_SIZE}, management::server_state::IDIssuer, protocol::{messages::{ArchivedProtocolMessage, ControlCode, ProtocolMessage, WebsocketMessage}, stream::SequencedTcpStream}, sync::{coordinator::AsyncCoordinator, distributed::{LightPacket, TunnelCenter}}};

pub const SERVER_CONTROL_PORT: u16 = 8001;
pub const WEB_SERVER_PORT: u16 = 8000;



pub struct KubeportServer {
    //pub listener: Arc<TcpListener>,

    pub link: DashMap<u16, Arc<TunnelCenter<u32, ProtocolMessage>>>,
    pub issuer: IDIssuer<u32>
    // pub map: DashMap<String, WebsocketProxy>
}




impl KubeportServer {

    pub fn create() -> Result<Self> {
        Ok(Self {
            link: DashMap::new(),
            issuer: IDIssuer::default(),
        })
    }
    pub async fn spin() -> Result<()> {
        configure_system_logger("logs");
        run_kubeport_server(Arc::new(Self::create()?)).await;
        Ok(())
    }
    // pub fn issue_new_id(&self) -> u32 {
    //     let mut thread_rng = rand::thread_rng();
    //     let mut gen: u32 = thread_rng.gen();
    //     while self.ids.contains(&gen) {
    //         gen = thread_rng.gen();
    //     }
    //     gen

    // }
   
}






pub async fn run_kubeport_server(state: Arc<KubeportServer>) {
    info!("Starting up the forwarding service...");


    // let center: TunnelCenter<u32, ProtocolMessage> = TunnelCenter::new(CHANNEL_SIZE);
    // center.to_tunnel(ProtocolMessage::Open(32)).await.unwrap();

    // Starts the websocket end of things.
    start_forwarding_channels(state).await;


}


pub async fn handle_tcp_pair(service_id: u16, state: Arc<KubeportServer>, conn: TcpStream, addr: SocketAddr) -> Result<()> {
    

    let channel_id_permit = state.issuer.issue();

    let channel_id = channel_id_permit.id();

    println!("New TCP pair with ID: {}", channel_id);

    let proxy = state.link.get(&service_id).unwrap().clone();

    proxy.to_tunnel(ProtocolMessage::Open(channel_id)).await?;

    let coordinator = Arc::new(AsyncCoordinator::new());

    let (mut r, mut s) = conn.into_split();
    

  


    // Read loop.
    tokio::spawn({
        let proxy = proxy.clone();
        let coordinator = Arc::clone(&coordinator);
        // let channel = id.clone();

        async move {
            loop {
                let buf = &mut [0u8; 4096];
                
                let b = match r.read(buf).await {
                    Ok(b) => b,
                    Err(e) => {
                        coordinator.shutdown();
                        break;
                    }
                };
                
                if b == 0 {
                    coordinator.shutdown();
                    break;
                }


                println!("| == PUSHING THROUGH TUNNEL");
                println!("| \tTEXT: {:?}", from_utf8(&buf[..b]));
                proxy.to_tunnel(ProtocolMessage::Message(WebsocketMessage {
                    id: channel_id,
                  // code: ControlCode::Neutral,
                    data: buf[..b].to_vec()
                })).await.unwrap();
            }
            println!("Read loop done.");
        }
    });

    // Write loop.
    tokio::spawn({
        let proxy = proxy.clone();

        let coordinator = Arc::clone(&coordinator);
        async move {
            loop {
                //let distro = proxy.subscribe(&id)
             
                let packet = tokio::select! {
                    p = proxy.subscribe(channel_id) => p.unwrap(),
                    _ = coordinator.wait_on_change() => break
                };

                if let ArchivedProtocolMessage::Message(message) = packet.access().unwrap() {
                    println!("| <== PULLING FROM TUNNEL");
                    println!("| Message: {:?}", from_utf8(&message.data));
                    s.write_all(&message.data).await.unwrap();
                }
               
               
            }
            println!("Write loop done.");
        }
    });
    Ok(())
}

pub async fn start_forwarding_channels(state: Arc<KubeportServer>) {




    // UDP tests
    
    // Working TCP
    let websocket_address = SocketAddr::from(([0, 0, 0, 0], WEB_SERVER_PORT));
    let listener = TcpListener::bind(&websocket_address).await.unwrap();
    info!("TCP server listening on {}...", websocket_address);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_server_conn(Arc::clone(&state), stream, addr));
    }
}

use anyhow::anyhow;


// tokio::spawn({
//     let state = Arc::clone(&state);
//     async move {
//         loop {

        
        
//             let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], WEB_SERVER_PORT + 1))).await.unwrap();
//             let (conn, addr) = listener.accept().await.unwrap();
//             tokio::spawn(handle_tcp_pair(Arc::clone(&state), conn, addr));

       
//         }
//     }
// });


pub async fn launch_tcp_server(state: Arc<KubeportServer>, listener: TcpListener) -> Result<()> {

    
    //             let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], WEB_SERVER_PORT + 1))).await.unwrap();
    info!("Waiting on {}...", listener.local_addr()?);
    loop {
        let (conn, addr) = listener.accept().await.unwrap();
        tokio::spawn(handle_tcp_pair(listener.local_addr()?.port(), Arc::clone(&state), conn, addr));

    }

    Ok(())
}

pub async fn handle_server_conn(state: Arc<KubeportServer>, stream: TcpStream, addr: SocketAddr) -> Result<()> {
    info!("Received connection from {addr}.");

    let (mut read, mut write) = SequencedTcpStream::split(stream);
    let packet_date = read.recv().await?;
    let packet = ProtocolMessage::from_bytes(&packet_date)?;
    let service_id;
    let service_name;
    if let ArchivedProtocolMessage::Establishment((id, name)) = packet {
        state.link.insert(*id, Arc::new(TunnelCenter::new(CHANNEL_SIZE)));
        info!("New service [{id}] started.");
        service_id = *id;
        service_name = name.to_string();
    } else {
        return Err(anyhow!("Did not receive an establishment packet."));
    }


    // Determine if the port is suitable.
    let listener = match TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], service_id as u16))).await {
        Ok(v) => v,
        Err(_) => TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?
    };

    write.send(&ProtocolMessage::Establishment((listener.local_addr()?.port(), service_name)).to_bytes()?).await?;

    tokio::spawn(launch_tcp_server(Arc::clone(&state), listener));




    tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            loop {
                let packet_data = read.recv().await.unwrap();
                let message_address;
                if let Ok(ArchivedProtocolMessage::Message(message)) = ProtocolMessage::from_bytes(&packet_data) {
                    message_address = message.id;
                    // if message.code == ControlCode::Close {
                    //     continue;
                    // }
                   
                } else {
                    continue
                }
                println!("RECEIVING A MESSAGE FROM BACKEND...");
                println!("MESSAGE: {:?}", from_utf8(packet_data.as_ref()));
                state.link.get(&service_id).unwrap().publish(message_address, packet_data).await.unwrap();
            }
        }
    });


    let service = Arc::clone(&state.link.get(&service_id).unwrap());
    loop {
        let packet = service.recv_from_tunnel().await?;
        write.send(packet.to_bytes()).await?;
    }
}
// use anyhow::anyhow;

pub fn frame_to_vec(frame: &Frame) -> AlignedVec {
    let mut o_vec = AlignedVec::new();
    o_vec.extend_from_slice(&frame.payload);
    o_vec
}

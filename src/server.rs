

use std::{borrow::BorrowMut, collections::{HashMap, HashSet}, net::SocketAddr, ops::Index, process::exit, str::from_utf8, sync::Arc, usize};

use anyhow::Result;

use dashmap::DashMap;


use rand::Rng;
use rkyv::AlignedVec;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream, UdpSocket}, sync::{Notify, OnceCell}};
use tracing::info;

use fastwebsockets::Frame;

use crate::{commons::{configure_system_logger, CHANNEL_SIZE}, protocol::{messages::{ArchivedProtocolMessage, ControlCode, ProtocolMessage, WebsocketMessage}, stream::SequencedStream}, sync::{coordinator::AsyncCoordinator, pubsub::Kraken}};

pub const SERVER_CONTROL_PORT: u16 = 8001;
pub const WEB_SERVER_PORT: u16 = 8000;


#[derive(Debug)]
pub struct KubeportServer {
    //pub listener: Arc<TcpListener>,

    pub link: DashMap<u16, Arc<Kraken>>,
    pub ids: HashSet<u32>
    // pub map: DashMap<String, WebsocketProxy>
}




impl KubeportServer {

    pub fn create() -> Result<Self> {
        Ok(Self {
            link: DashMap::new(),
            ids: HashSet::new(),
        })
    }
    pub async fn spin() -> Result<()> {
        configure_system_logger("logs");
        run_kubeport_server(Arc::new(Self::create()?)).await;
        Ok(())
    }
    pub fn issue_new_id(&self) -> u32 {
        let mut thread_rng = rand::thread_rng();
        let mut gen: u32 = thread_rng.gen();
        while self.ids.contains(&gen) {
            gen = thread_rng.gen();
        }
        gen

    }
   
}






pub async fn run_kubeport_server(state: Arc<KubeportServer>) {

    info!("Configuring Axum service...");


    // tokio::spawn(quic_server());

    



    // Starts the websocket end of things.
    start_websocket_server(state).await;


}


pub async fn handle_tcp_pair(service_id: u16, state: Arc<KubeportServer>, conn: TcpStream, addr: SocketAddr) -> Result<()> {
    

    let channel_id = state.issue_new_id();
    println!("New TCP pair with ID: {channel_id}");

    let proxy = state.link.get(&service_id).unwrap().clone();

    proxy.send_primary(ProtocolMessage::Open(channel_id)).await.unwrap();

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
                proxy.send_primary(ProtocolMessage::Message(WebsocketMessage {
                    id: channel_id,
                    code: ControlCode::Neutral,
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

                if let ArchivedProtocolMessage::Message(message) = ProtocolMessage::from_bytes(&packet).unwrap() {
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

pub async fn start_websocket_server(state: Arc<KubeportServer>) {


    
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

    let (mut read, mut write) = SequencedStream::split(stream);
    let packet_date = read.recv().await?;
    let packet = ProtocolMessage::from_bytes(&packet_date)?;
    let service_id;
    let service_name;
    if let ArchivedProtocolMessage::Establishment((id, name)) = packet {
        state.link.insert(*id, Arc::new(Kraken::new(CHANNEL_SIZE)));
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

    write.send(ProtocolMessage::Establishment((listener.local_addr()?.port(), service_name)).to_bytes()?).await?;

    tokio::spawn(launch_tcp_server(Arc::clone(&state), listener));




    tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            loop {
                let packet_data = read.recv().await.unwrap();
                let message_address;
                if let Ok(ArchivedProtocolMessage::Message(message)) = ProtocolMessage::from_bytes(&packet_data) {
                    message_address = message.id;
                    if message.code == ControlCode::Close {
                        continue;
                    }
                   
                } else {
                    continue
                }
                println!("RECEIVING A MESSAGE FROM BACKEND...");
                println!("MESSAGE: {:?}", from_utf8(packet_data.as_ref()));
                state.link.get(&service_id).unwrap().publish(&message_address, packet_data).await.unwrap();
            }
        }
    });


    let service = Arc::clone(&state.link.get(&service_id).unwrap());
    loop {
        let packet = service.recv_primary().await?;
        write.send(packet).await?;
    }
    
    // loop {
    //     let packet_date = sequenced.recv().await?;

    
    // }



    
  


    Ok(())
}

// use anyhow::anyhow;

pub fn frame_to_vec(frame: &Frame) -> AlignedVec {
    let mut o_vec = AlignedVec::new();
    o_vec.extend_from_slice(&frame.payload);
    o_vec
}

// async fn handle_client(state: Arc<KubeportServer>, fut: UpgradeFut) -> Result<()> {

//     // info!("Aware of Server: {:?}", SERVER_STATE.get().is_some());


//     let (ws_read, mut ws_write) = fut.await?.split(tokio::io::split);

//     let mut ws_read = FragmentCollectorRead::new(ws_read);
    
//     // let mut ws: FragmentCollector<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>> = FragmentCollector::new(fut.await?);
    

//     // Wait for the establishment message.
//     let service_id;
//     let frame = ws_read.read_frame::<_, WebSocketError>(&mut move |_| async { unreachable!() }).await?;
//     let frame_vec = frame_to_vec(&frame);
//     // println!("Frame: {:?}", &frame.payload[..]);
//     match ProtocolMessage::from_bytes(&frame_vec)? {
//         ArchivedProtocolMessage::Establish(stream) => {
//             info!("Tried to establish as {}", stream);
//             service_id = *stream;
//         },
//         _ => {
//             error!("The client did not send the establishment packet first.");
//             return Err(anyhow!("Improper connection handling."))
//         }
//     }
//     info!("Got an establishment request succesfully. Size: {}", frame_vec.len());

//     // Open a service.
//     // TODO: Cancel if existing service.
//     let service = Arc::new(Kraken::new(CHANNEL_SIZE));
//     state.link.insert(service_id, Arc::clone(&service));

//     println!("Chck: {:?}", state.link.contains_key(&service_id));



//     tokio::spawn({

//         async move {
//             loop {
//                 let frame = ws_read.read_frame::<_, WebSocketError>(&mut move |_| async { unreachable!() }).await.unwrap();
//                 match frame.opcode {
//                     OpCode::Close => break,
//                     OpCode::Text | OpCode::Binary => {
//                             let aligned = frame_to_vec(&frame);
//                         let m_id;
//                         if let ArchivedProtocolMessage::Message(m) = ProtocolMessage::from_bytes(&aligned).unwrap() {
//                             m_id = m.id;
//                         } else {
//                             continue
//                         }
//                         state.link.get(&service_id).unwrap().publish(&m_id, aligned).await.unwrap();
//                     }
//                     _ => {}
//                 }
//             }
//         }
//     });

//     loop {
//         let packet = service.recv_primary().await?;
//         ws_write.write_frame(Frame::binary(Payload::Borrowed(packet.as_ref()))).await?;
//     }

   
  
//     Ok(())
//   }


// async fn server_upgrade(
//     state: Arc<KubeportServer>,
//     mut req: Request<Incoming>,
//   ) -> Result<Response<Empty<Bytes>>, WebSocketError> {
//     let (response, fut) = upgrade::upgrade(&mut req)?;
  
//     tokio::task::spawn(async move {
//       if let Err(e) = tokio::task::unconstrained(handle_client(state, fut)).await {
//         eprintln!("Error in websocket connection: {}", e);
//       }
//     });
  
//     Ok(response)
//   }

// pub async fn handle_ws_conn(state: Arc<KubeportServer>, stream: TcpStream, addr: SocketAddr) {
//     //let mut ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
//     let io = hyper_util::rt::TokioIo::new(stream);
//     let conn_fut = http1::Builder::new().serve_connection(io, hyper::service::service_fn({

//         |req| server_upgrade(Arc::clone(&state), req)
//     })).with_upgrades();
//     if let Err(e) = conn_fut.await {
//         error!("Failed to get a connection with error: {e}.");
//     }
// }

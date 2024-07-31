use std::{borrow::BorrowMut, net::SocketAddr, ops::Index, process::exit, str::from_utf8, sync::Arc, usize};

use anyhow::Result;

use dashmap::DashMap;

use futures_util::{FutureExt, SinkExt, StreamExt};






use rkyv::AlignedVec;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::{Notify, OnceCell}};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{error, info, instrument::WithSubscriber};
use uuid7::{uuid7, Uuid};

use fastwebsockets::{upgrade::{self, UpgradeFut}, FragmentCollector, Frame, Payload};
use fastwebsockets::OpCode;
use fastwebsockets::WebSocketError;
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;


use crate::{commons::{configure_system_logger, WebsocketProxy, CHANNEL_SIZE}, protocol::messages::{ArchivedProtocolMessage, ControlCode, ProtocolMessage, WebsocketMessage}, sync::{coordinator::AsyncCoordinator, pubsub::Kraken}};

pub const SERVER_CONTROL_PORT: u16 = 8001;
pub const WEB_SERVER_PORT: u16 = 8000;


#[derive(Debug)]
pub struct KubeportServer {
    //pub listener: Arc<TcpListener>,
    pub link: DashMap<u32, Arc<Kraken>>,
    // pub map: DashMap<String, WebsocketProxy>
}



impl KubeportServer {

    pub fn create() -> Result<Self> {
        Ok(Self {
            link: DashMap::new(),
            // map: DashMap::new()
        })
    }
    pub async fn spin() -> Result<()> {
        configure_system_logger("logs");
   
        // SERVER_STATE.set(Self::create().unwrap()).unwrap();
        // println!("Set.");

        // SERVER_STATE.set(Arc::new(Self::create()?)).unwrap();
        
        // println!("Aware of Server: {:?}", SERVER_STATE);
        // tokio::spawn({
        //     let server = Arc::clone(&server);
        //     let listener = Arc::clone(&server.listener);
        //     async move {
        //         run_server_listener(server, listener).await;
        //     }
        // });

        run_kubeport_server(Arc::new(Self::create()?)).await;
        Ok(())
    }
   
}






pub async fn run_kubeport_server(state: Arc<KubeportServer>) {

    info!("Configuring Axum service...");


    // tokio::spawn(quic_server());

    tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            loop {

            
            
                let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], WEB_SERVER_PORT + 1))).await.unwrap();
                let (conn, addr) = listener.accept().await.unwrap();
                tokio::spawn(handle_tcp_pair(Arc::clone(&state), conn, addr));

           
            }
        }
    });



    // Starts the websocket end of things.
    start_websocket_server(state).await;


}


pub async fn handle_tcp_pair(state: Arc<KubeportServer>, conn: TcpStream, addr: SocketAddr) -> Result<()> {
    
    let id = 0;

    let proxy = state.link.get(&id).unwrap().clone();

    proxy.send_primary(ProtocolMessage::Open(id.clone())).await.unwrap();

    let coordinator = Arc::new(AsyncCoordinator::new());

    let (mut r, mut s) = conn.into_split();
    

  


    // Read loop.
    tokio::spawn({
        let proxy = proxy.clone();
        let coordinator = Arc::clone(&coordinator);
        let id = id.clone();
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


                println!("| <- RECEIVING");
                proxy.send_primary(ProtocolMessage::Message(WebsocketMessage {
                    id: id.clone(),
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
                println!("| -> SENDING");
                let packet = tokio::select! {
                    p = proxy.subscribe(id) => p.unwrap(),
                    _ = coordinator.wait_on_change() => break
                };
                s.write_all(packet.as_ref()).await.unwrap();
               
            }
            println!("Write loop done.");
        }
    });


        // }
    // });

    Ok(())

}

pub async fn start_websocket_server(state: Arc<KubeportServer>) {
    
    let websocket_address = SocketAddr::from(([0, 0, 0, 0], WEB_SERVER_PORT));
    let listener = TcpListener::bind(&websocket_address).await.unwrap();
    info!("Websocket server listening on {}", websocket_address);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_ws_conn(Arc::clone(&state), stream, addr));
    }
}



use anyhow::anyhow;

pub fn frame_to_vec(frame: &Frame) -> AlignedVec {
    let mut o_vec = AlignedVec::new();
    o_vec.extend_from_slice(&frame.payload);
    o_vec
}

async fn handle_client(state: Arc<KubeportServer>, fut: UpgradeFut) -> Result<()> {

    // info!("Aware of Server: {:?}", SERVER_STATE.get().is_some());


    let mut ws: FragmentCollector<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>> = FragmentCollector::new(fut.await?);
    

    // Wait for the establishment message.
    let service_id;
    let frame = ws.read_frame().await?;
    let frame_vec = frame_to_vec(&frame);
    // println!("Frame: {:?}", &frame.payload[..]);
    match ProtocolMessage::from_bytes(&frame_vec)? {
        ArchivedProtocolMessage::Establish(stream) => {
            info!("Tried to establish as {}", stream);
            service_id = stream;
        },
        _ => {
            error!("The client did not send the establishment packet first.");
            return Err(anyhow!("Improper connection handling."))
        }
    }
    info!("Got an establishment request succesfully.");

    // Open a service.
    // TODO: Cancel if existing service.
    let service = Arc::new(Kraken::new(CHANNEL_SIZE));
    state.link.insert(*service_id, Arc::clone(&service));

    println!("Chck: {:?}", state.link.contains_key(service_id));



    loop {

      //  let service = SERVER_STATE.get().unwrap().link.get(&service_id).unwrap();
        tokio::select! {
            frame = ws.read_frame() => {
                let frame = frame?;
                match frame.opcode {
                    OpCode::Close => break,
                    OpCode::Text | OpCode::Binary => {
                          let aligned = frame_to_vec(&frame);
                        let m_id;
                        if let ArchivedProtocolMessage::Message(m) = ProtocolMessage::from_bytes(&aligned)? {
                            m_id = m.id;
                        } else {
                            continue
                        }
                        state.link.get(&service_id).unwrap().publish(&m_id, aligned).await?;
                    }
                    _ => {}
                }
            }
            packet = service.recv_primary() => {
                ws.write_frame(Frame::binary(Payload::Borrowed(packet?.as_ref()))).await?;

            }
        }
        


        
        
       
        
    }
  
    Ok(())
  }


async fn server_upgrade(
    state: Arc<KubeportServer>,
    mut req: Request<Incoming>,
  ) -> Result<Response<Empty<Bytes>>, WebSocketError> {
    let (response, fut) = upgrade::upgrade(&mut req)?;
  
    tokio::task::spawn(async move {
      if let Err(e) = tokio::task::unconstrained(handle_client(state, fut)).await {
        eprintln!("Error in websocket connection: {}", e);
      }
    });
  
    Ok(response)
  }

pub async fn handle_ws_conn(state: Arc<KubeportServer>, stream: TcpStream, addr: SocketAddr) {
    //let mut ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
    let io = hyper_util::rt::TokioIo::new(stream);
    let conn_fut = http1::Builder::new().serve_connection(io, hyper::service::service_fn({

        |req| server_upgrade(Arc::clone(&state), req)
    })).with_upgrades();
    if let Err(e) = conn_fut.await {
        error!("Failed to get a connection with error: {e}.");
    }
    
    // info!("Established a WebSocket connection on {}", addr);

    // if let Some(ProtocolMessage::Establish(stream)) = if let Message::Binary(contents) = ws_stream.next().await.unwrap().unwrap() {
    //     ProtocolMessage::deserialize(&contents).ok()
    // } else {
    //     None
    // } {
    //     info!("Received establishment packet. Requesting a service ID of {stream}.");


            
    //     info!("Starting a reverse proxy for stream [{}]...", stream);
    //     state.map.insert(stream.clone(), WebsocketProxy::create(ws_stream));
    //     info!("Started a reverse proxy for stream [{}]...", stream);

    //     state.map.get(&stream).unwrap().get_coordinator().wait_on_change().await;
    //     info!("Detected shutdown for stream [{}]...", stream);
    //     state.map.remove(&stream);
    //     info!("Cleaned up stream [{}]", stream);


    // } else {
    //     error!("Did not receive the establish packet. Closing the connection.");
    //     return;
    // }

    // ws_stream.send(Message::Text(String::from("hello!"))).await.unwrap();








}

// fn parse_path(path: &str) -> (&str, &str) {
//     match path[1..].find(|c| c == '/') {
//         Some(v) => (&path[1..v + 1], &path[v + 1..]),
//         None => (&path[1..], "/")
//     }
//     // let splitted = path[1..].find(|c| c == '/')



// }

// async fn handle_request(State(state): State<Arc<KubeportServer>>, req: Request) -> Response {
//     let (mut parts, body) = req.into_parts();


//     // println!("SERVICE {} {}", service, path);

    
//     let full_path = parts.uri.to_string();

//     let (endpoint, modified_path) = parse_path(&full_path);

//     println!("MODIFIED PATH: {}", modified_path);

//    // parts.headers.insert(HOST, "localhost:4032".parse().unwrap());

//     let request_line = format!("{} {} HTTP/1.1\r\n", parts.method, modified_path);

//     let headers = parts.headers.iter().map(|(name, value)| format!("{name}: {}\r\n", value.to_str().unwrap())).collect::<String>();

//     let body_bytes = body::to_bytes(Body::from(body), usize::MAX).await.unwrap();
//     let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

//     let final_request = format!("{}{}{}\r\n", request_line, headers, body_str);
//     println!("REQUEST BODY: {}", final_request);

//     println!("REQUESTED ENDPOINT: {}", endpoint);


//     let bro = state.map.get(endpoint).unwrap().value().clone();
//     let id = uuid7();
//   //  bro.send_main(ProtocolMessage::Open(id.clone())).await.unwrap();
//     bro.send_main(ProtocolMessage::Message(WebsocketMessage { 
//         id: id.clone(),
//         code: ControlCode::Open,
//         data: final_request.as_bytes().to_vec()

//     })).boxed().await.unwrap();


//     //let chan = bro.get_channel(&id).await;
//     //let chan = bro.get_distributor();
//     let mut resp = Vec::new();
//     loop {
//         let msg = bro.get_distributor().subscribe(&id).await.unwrap();
//         //if let Ok(msg) = chan.recv().await {
//             if msg.code == ControlCode::Close {
//                 println!("SAW THE CONTROL CODE...");
//                 break;
//             }
//             println!("RECEIVING PACKET...");
//             resp.extend_from_slice(&msg.data);
//         // } else {
//         //     // TODO: Throw error.
//         //     println!("Failed.");
//         //     break
//         // }
//     }
//     println!("OUT OF LOOPPP");
    
//     // loop {
//     //     if let Ok(packet) = chan.recv().await {
//     //         match packet {

//     //         }
//     //     }
//     // }

//     // let resp = bro.get_channel(&id).await.recv().await.unwrap();

//     // println!("BRO {:#?}", resp);



    
//     let mut headers = [httparse::EMPTY_HEADER; 64];
//     let mut response = httparse::Response::new(&mut headers);
//     response.parse(&resp).unwrap();


 
//     println!("BRO: {:?}", from_utf8(&resp).unwrap().to_string());



//     let mut body: &[u8] = &[];
//     for i in 0..(resp.len() - 3) {
//         if resp[i..i+4] == [13, 10, 13, 10] {
//             body = &resp[i+4..];
//             break
//         }
//     }


//     // TODO: Missing parts of request.
    
//     let mut hyper_response = hyper::Response::builder()
//         .status(response.code.unwrap());
//     for header in response.headers {
//         hyper_response = hyper_response.header(header.name, header.value);
//     }
//     let hyper_response = hyper_response.body(Body::from(body.to_vec())).unwrap();


    


//     // hyper_response

//     hyper_response

// }

// async fn check_availability(State(state): State<Arc<KubeportServer>>, Path(name): Path<String>) -> (StatusCode, String)  {
//     (StatusCode::ACCEPTED, (!state.has_route(&name)).to_string())
// }


// async fn websocket_handler(Path(pathname): Path<String>, ws: WebSocketUpgrade, State(state): State<Arc<KubeportServer>>) -> Response {
//     ws.on_upgrade(|socket| handle_websocket_connection(pathname, socket, state))
// }

// async fn handle_websocket_connection(stream: String, ws: WebSocket, state: Arc<KubeportServer>) {
//     info!("Starting a reverse proxy for stream [{}]...", stream);
//     state.map.insert(stream.clone(), WebsocketProxy::create(ws));
//     info!("Started a reverse proxy for stream [{}]...", stream);

//     state.map.get(&stream).unwrap().get_coordinator().wait_on_change().await;
//     info!("Detected shutdown for stream [{}]...", stream);
//     state.map.remove(&stream);
//     info!("Cleaned up stream [{}]", stream);




// }


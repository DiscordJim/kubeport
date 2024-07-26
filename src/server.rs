use std::{net::SocketAddr, ops::Index, process::exit, str::from_utf8, sync::Arc};

use anyhow::Result;
use axum::{body::{Body, Bytes}, extract::{ws::{Message, WebSocket}, FromRequest, Host, Path, RawForm, Request, State, WebSocketUpgrade}, http::{request, StatusCode}, response::Response, routing::{any, get, post}, Router};
use dashmap::DashMap;
use futures_util::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::Notify};
use uuid7::{uuid7, Uuid};

use crate::{commons::{ProtocolMessage, WebsocketMessage, WebsocketProxy}, httptools::basic_response};

pub const SERVER_CONTROL_PORT: u16 = 8001;
pub const WEB_SERVER_PORT: u16 = 8000;



#[derive(Clone)]
pub struct KubeportServer {
    pub listener: Arc<TcpListener>,
    pub map: DashMap<String, WebsocketProxy>
}

impl KubeportServer {

    pub async fn create() -> Result<Self> {

        let addr = SocketAddr::from(([127,0,0,1], SERVER_CONTROL_PORT));
        let listener = Arc::new(TcpListener::bind(&addr).await?);


       

        Ok(KubeportServer {
            listener,
            map: DashMap::new()
        })
    }
    pub async fn spin() -> Result<()> {
        let server = Arc::new(KubeportServer::create().await?);
        tokio::spawn({
            let server = Arc::clone(&server);
            let listener = Arc::clone(&server.listener);
            async move {
                run_server_listener(server, listener).await;
            }
        });

        run_kubeport_server(server).await;
        Ok(())
    }
    pub fn has_route(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }
}

use anyhow::{anyhow};




pub async fn handle_raw_request(id: &Uuid, state: Arc<KubeportServer>, mut stream: TcpStream) -> Result<()> {

    // Get the top of the request.
    let mut buf: &mut [u8] = &mut [0u8; 18_000];
    let bytes_read = stream.peek(buf).await?;
    buf = &mut buf[..bytes_read];

    // let pos = buf.index(index)

    println!("SEQUENCE");

    let slice_position = buf.iter().position(|r| *r == 10).ok_or_else(|| anyhow!("Failed to scan the request."))? - 1;
    let rest_of_response = &buf[slice_position..];

    let mut tip = from_utf8(&buf[..slice_position])?.split(" ");

    println!("Wow: {:?}", tip.nth(1));


    

     // Move this into another file.
     let mut headers = [httparse::EMPTY_HEADER; 64];
     let mut parsed = httparse::Request::new(&mut headers);
     parsed.parse(buf)?;

     println!("RAW: {}", from_utf8(buf).unwrap());
    println!("Request: {:?}", parsed);

     // path
     let mut path = parsed.path.unwrap();
     if path.starts_with("/") {
        path = &path[1..];
     }
     if let Some(item) = path.split("/").next() {
        if !state.has_route(item) {
            stream.write_all(basic_response(StatusCode::BAD_REQUEST).as_bytes()).await?;
            return Ok(())
        }
        //accept_connection(stream, state.map.get(item).unwrap().value().clone(), id.to_owned()).await?;
        //state.map.get(item).unwrap().send_main(ProtocolMessage::Message())
     } else {   
        stream.write_all(basic_response(StatusCode::BAD_REQUEST).as_bytes()).await?;
     }
     


     println!("Received a request {:?}", parsed);

    Ok(())
}

pub async fn run_server_listener(state: Arc<KubeportServer>, server: Arc<TcpListener>) {
    loop {
        let stream = server.accept().await.unwrap().0;

        tokio::spawn({
            let id = uuid7();
            let state = Arc::clone(&state);
            
            //let server = Arc::clone(&server);
            async move {

                

                handle_raw_request(&id, state, stream).await.unwrap();

                //accept_connection(stream, state.map.get("name").unwrap().clone(), id).await.unwrap();
                

                //let mut buffer = [0u8; 4096];
                //let rd = stream.read(&mut buffer).await.unwrap();
                //handle_raw_request(&id, state, stream, &buffer[..rd]).boxed().await.unwrap();

               
            }
        });
    }
}

pub async fn run_kubeport_server(server: Arc<KubeportServer>) {

    let app = Router::new()
       // .route("/*path", any(handle_request))
        .route("/forward/:service", get(websocket_handler))
        .route("/available/:name", post(check_availability))
       // .route("/forward/:service", post(connect_to_route))
        .with_state(server);

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], WEB_SERVER_PORT))).await.unwrap();
    axum::serve(listener, app).await.unwrap();


}

// async fn handle_request(body: RawForm) -> String {
//     println!("REQUEST BODY: {:?} {:?}", 3, 3);

//     let request: &str = Request::builder().into();
//     println!("REQUEST {:?}", request);

//     "Hello".to_string()

// }

async fn check_availability(State(state): State<Arc<KubeportServer>>, Path(name): Path<String>) -> (StatusCode, String)  {


    (StatusCode::ACCEPTED, (!state.has_route(&name)).to_string())
}


async fn websocket_handler(Path(pathname): Path<String>, ws: WebSocketUpgrade, State(state): State<Arc<KubeportServer>>) -> Response {
    ws.on_upgrade(|socket| handle_websocket_connection(pathname, socket, state))
}




// async fn send_protocol_message(ws: &mut WebSocket, message: ProtocolMessage) -> Result<()> {
//     ws.send(Message::Binary(message.serialize().await?)).await?;

//     Ok(())
// }

async fn handle_websocket_connection(stream: String, ws: WebSocket, state: Arc<KubeportServer>) {

    
        // println!("Stream: {:?}", stream);

    
    // let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 54314))).await.unwrap();
    // println!("Created a listener on {:?}", listener.local_addr());


    println!("Started a reverse proxy for stream {}", stream);
    state.map.insert(stream.clone(), WebsocketProxy::create(ws));
    // let proxy = state.map.get(&stream).unwrap().value().clone();

   
    // println!("Proxy");
   

    // loop {
    //     let t_stream = listener.accept().await.unwrap().0;
    //     let id = uuid7();
    //     println!("Found a stream");
    //     println!("Got a websocket request. Opening under ID {}.", id);
    //     proxy.send_main(ProtocolMessage::Open(id.clone())).await.unwrap();
    //     //send_protocol_message(&mut ws, ProtocolMessage::Open(id)).await.unwrap();
    //     tokio::spawn({

    //         let proxy = proxy.clone();
    //         let id = id.clone();
    //         async move {
    //             accept_connection(t_stream, proxy, id).await.unwrap();
    //             println!("CONNECTION CLEANED");
    //         }
    //     });
    
    // }

}

async fn accept_connection(t_stream: TcpStream, proxy: WebsocketProxy, id: Uuid, initial: &mut [u8]) -> Result<()> {
    
    println!("GOT A CONNECTION");

    proxy.send_main(ProtocolMessage::Open(id.clone())).await?;

    let (mut t_read, mut t_write) = t_stream.into_split();

    let channel = proxy.get_channel(&id).await;
    tokio::spawn({

        let proxy = proxy.clone();
        let channel = Arc::clone(&channel);
        async move {

            loop {
                println!("Waiting...");
                let mut buffer = [0u8; 4096];
                match t_read.read(&mut buffer).await {
                    Ok(b) => {
                        if b == 0 {
                            channel.shutdown.shutdown();
                            break
                        }
                        println!("| -> Pushing {b}.");
                        println!("| -> Data Pushed: {}", from_utf8(&buffer[..b]).unwrap());

                        proxy.send_main(ProtocolMessage::Message(WebsocketMessage {
                            id: id.clone(),
                            data: buffer[..b].to_vec()
                        })).await.unwrap();
                    },
                    Err(_) => {
                        println!("Shutting down control loop (READ)...");
                        break
                    }
                }

            }

        }
    });
    
// let shutdown = Arc::new(Notify::new());


    loop {
        println!("Waiting on message...");
        match channel.recv().await {
            Ok(mut v) => {
                println!("| <- Writing {}", v.data.len());
                t_write.write_all(&mut v.data).await.unwrap()
            },
            Err(_) => break
        }
    }
 

    println!("Staritng loop...");
   
    println!("Fully terminated.");
    Ok(())
}


// loop {
//     println!("Waiting for websocket message...");

  

//     match proxy.recv(&id).await {
//          Ok(mut m) => {
//              if m.data.len() == 0 {
//                  shutdown.notify_waiters();
//                  break
//              }
//              println!("|| Writing bytes to stream");
//              t_write.write_all(&mut m.data).await.unwrap()
//          },
//          Err(e) => {
//              println!("Failed because of {e}");
//          }
//     }

//     println!("Shutdown RR");
    
//      // if let mut wow = proxy.recv(&id).await.unwrap() {
//      //     println!("Send: {:#?}", wow);
//      //     t_write.write_all(&mut wow.data).await.unwrap();
//      //     exit(1);
//      // }
//  }

   // println!("Booting threads");
    // tokio::spawn({
    //     let proxy = proxy.clone();
    //     // let shutdown = Arc::clone(&shutdown);
    //     println!("Cloned the proxy...");

    //     async move {
    //         println!("Listeneing for read...");
        
    //         loop {
    //             println!("Waiting on read...");
    //             let mut buffer = [0u8; 4096];
    //             let b = tokio::select! {
    //                 m = t_read.read(&mut buffer) => m,
    //                 _ = shutdown.notified() => break
    //             }.unwrap();
    //             if b == 0 {
    //                 println!("Connection ending. Shutting down.");
    //                 shutdown.notify_waiters();
    //                 break;
    //                 // exit(1);
    //             }
                
    //             println!("Received some bytes. Forwarding them. {}", b);
    //             proxy.send(&id, buffer[..b].to_vec()).await.unwrap();
    //         }
    // }});


// async fn connect_to_service(State(state): State<KubeportServer>, Path(service): Path<String>) -> (StatusCode, String) {

// }

// async fn connect_to_route(State(state): State<KubeportServer>, Path(service): Path<String>, body: Bytes) -> (StatusCode, String) {
//     // println!("Waiting on bro");
//     // let mut bro = state.listener.accept().await.unwrap().0;

//     // let mut boof = [0u8; 4];
//     // println!("Waiting on read");
//     // let buf = bro.read(&mut boof).await.unwrap();


//     // println!("Bro: {:?}", boof);

//     println!("Hello: {:?}", body);

//     let conn = TcpStream::connect("127.0.0.1:7969").await.unwrap();
//     println!("CONNECTED");
    

//     (StatusCode::ACCEPTED, "hello".to_string())
// }


use std::{net::SocketAddr, process::exit, str::from_utf8, sync::Arc};

use anyhow::Result;
use axum::{body::Bytes, extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, http::StatusCode, response::Response, routing::{get, post}, Router};
use dashmap::DashMap;
use futures_util::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::Notify};
use uuid7::{uuid7, Uuid};

use crate::commons::{ProtocolMessage, WebsocketMessage, WebsocketProxy};

pub const SERVER_CONTROL_PORT: u16 = 7969;

#[derive(Clone)]
pub struct KubeRoute {

}


#[derive(Clone)]
pub struct KubeportServer {
    pub listener: Arc<TcpListener>,
    pub map: DashMap<String, KubeRoute>
}

impl KubeportServer {

    pub async fn create() -> Result<Self> {

        let addr = SocketAddr::from(([127,0,0,1], SERVER_CONTROL_PORT));
        let listener = Arc::new(TcpListener::bind(&addr).await?);


        tokio::spawn({
            let listener = Arc::clone(&listener);
            async move {
                run_server_listener(listener).await;
            }
        });

        Ok(KubeportServer {
            listener,
            map: DashMap::new()
        })
    }
    pub async fn spin() -> Result<()> {
        

        run_kubeport_server(KubeportServer::create().await?).await;
        Ok(())
    }
    pub fn has_route(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }
}

pub async fn run_server_listener(server: Arc<TcpListener>) {
    loop {
        let (stream, addr) = server.accept().await.unwrap();
        println!("Stream connected: {:?}", addr);
    }
}

pub async fn run_kubeport_server(server: KubeportServer) {

    let app = Router::new()
        .route("/forward/:service", get(handle_ws))
        .route("/available/:name", post(check_availability))
       // .route("/forward/:service", post(connect_to_route))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();


}

async fn check_availability(State(state): State<KubeportServer>, Path(name): Path<String>) -> (StatusCode, String)  {


    (StatusCode::ACCEPTED, (!state.has_route(&name)).to_string())
}

async fn handle_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_websocket_connection)
}



// async fn send_protocol_message(ws: &mut WebSocket, message: ProtocolMessage) -> Result<()> {
//     ws.send(Message::Binary(message.serialize().await?)).await?;

//     Ok(())
// }

async fn handle_websocket_connection(ws: WebSocket) {

    
    

    
    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 54314))).await.unwrap();
    println!("Created a listener on {:?}", listener.local_addr());



    let proxy = WebsocketProxy::create(ws);

   
    println!("Proxy");
   

    loop {
        let t_stream = listener.accept().await.unwrap().0;
        let id = uuid7();
        println!("Found a stream");
        println!("Got a websocket request. Opening under ID {}.", id);
        proxy.send_main(ProtocolMessage::Open(id.clone())).await.unwrap();
        //send_protocol_message(&mut ws, ProtocolMessage::Open(id)).await.unwrap();
        tokio::spawn({

            let proxy = proxy.clone();
            let id = id.clone();
            async move {
                accept_connection(t_stream, proxy, id).await.unwrap();
                println!("CONNECTION CLEANED");
            }
        });
    
    }

}

async fn accept_connection(t_stream: TcpStream, proxy: WebsocketProxy, id: Uuid) -> Result<()> {
    
    println!("GOT A CONNECTION");

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


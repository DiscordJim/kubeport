use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::{body::Bytes, extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, http::StatusCode, response::Response, routing::{get, post}, Router};
use dashmap::DashMap;
use futures_util::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};
use uuid7::{uuid7, Uuid};

use crate::commons::{ProtocolMessage, WebsocketProxy};

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



async fn send_protocol_message(ws: &mut WebSocket, message: ProtocolMessage) -> Result<()> {
    ws.send(Message::Binary(message.serialize().await?)).await?;

    Ok(())
}

async fn handle_websocket_connection(mut ws: WebSocket) {

    let id = uuid7();
    println!("Got a websocket request. Opening under ID {:?}", id);

    send_protocol_message(&mut ws, ProtocolMessage::Open(Uuid::default())).await.unwrap();
    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await.unwrap();
    println!("Created a listener on {:?}", listener.local_addr());



    let proxy = WebsocketProxy::create(ws);

   
   

    loop {
        let t_stream = listener.accept().await.unwrap().0;
        tokio::spawn({

            let proxy = proxy.clone();
            let id = id.clone();
            async move {
                accept_connection(t_stream, proxy, id).await.unwrap();
            }
        });
    
    }
    


    println!("Done");

}

async fn accept_connection(t_stream: TcpStream, proxy: WebsocketProxy, id: Uuid) -> Result<()> {
    
    println!("GOT A CONNECTION");

    let (mut t_read, mut t_write) = Box::leak(Box::new(t_stream)).split();
    

    tokio::spawn({
        let proxy = proxy.clone();

        async move {
        
        loop {
            let mut buffer = [0u8; 4096];
            let b = t_read.read(&mut buffer).await.unwrap();
            //println!("Received some bytes. Forwarding them. {}", b);
            proxy.send(&id, buffer[..b].to_vec()).await.unwrap();
        }
    }});



    loop {
        if let Ok(mut wow) = proxy.recv(&id).await {
            t_write.write_all(&mut wow).await.unwrap();
        }
    }
}

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


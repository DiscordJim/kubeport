use std::{net::SocketAddr, ops::Index, process::exit, str::from_utf8, sync::Arc, usize};

use anyhow::Result;
use axum::{body::{self, Body, Bytes}, extract::{ws::{Message, WebSocket}, FromRequest, Host, Path, RawForm, Request, State, WebSocketUpgrade}, http::{request, StatusCode}, response::{IntoResponse, Response}, routing::{any, get, post}, Router};
use dashmap::DashMap;
use futures_util::{FutureExt, SinkExt, StreamExt};
use httparse::EMPTY_HEADER;
use hyper::header::HOST;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::Notify};
use tracing::info;
use uuid7::{uuid7, Uuid};

use crate::{commons::{configure_system_logger, ControlCode, ProtocolMessage, WebsocketMessage, WebsocketProxy}, httptools::basic_response};

pub const SERVER_CONTROL_PORT: u16 = 8001;
pub const WEB_SERVER_PORT: u16 = 8000;



#[derive(Clone)]
pub struct KubeportServer {
    //pub listener: Arc<TcpListener>,
    pub map: DashMap<String, WebsocketProxy>
}

impl KubeportServer {

    pub async fn create() -> Result<Self> {
        Ok(Self {
            map: DashMap::new()
        })
    }
    pub async fn spin() -> Result<()> {
        configure_system_logger("logs");
        let server = Arc::new(Self::create().await?);
        // tokio::spawn({
        //     let server = Arc::clone(&server);
        //     let listener = Arc::clone(&server.listener);
        //     async move {
        //         run_server_listener(server, listener).await;
        //     }
        // });

        run_kubeport_server(server).await;
        Ok(())
    }
    pub fn has_route(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }
}






pub async fn run_kubeport_server(server: Arc<KubeportServer>) {

    info!("Configuring Axum service...");

    let app = Router::new()
     .route("/*path", any(handle_request))
        .route("/forward/:service", get(websocket_handler))
        .route("/available/:name", post(check_availability))
       // .route("/forward/:service", post(connect_to_route))
        .with_state(server);

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], WEB_SERVER_PORT))).await.unwrap();
    axum::serve(listener, app).await.unwrap();


}

fn parse_path(path: &str) -> (&str, &str) {
    match path[1..].find(|c| c == '/') {
        Some(v) => (&path[1..v + 1], &path[v + 1..]),
        None => (&path[1..], "/")
    }
    // let splitted = path[1..].find(|c| c == '/')



}

async fn handle_request(State(state): State<Arc<KubeportServer>>, req: Request) -> Response {
    let (mut parts, body) = req.into_parts();


    // println!("SERVICE {} {}", service, path);

    
    let full_path = parts.uri.to_string();

    let (endpoint, modified_path) = parse_path(&full_path);

    println!("MODIFIED PATH: {}", modified_path);

   // parts.headers.insert(HOST, "localhost:4032".parse().unwrap());

    let request_line = format!("{} {} HTTP/1.1\r\n", parts.method, modified_path);

    let headers = parts.headers.iter().map(|(name, value)| format!("{name}: {}\r\n", value.to_str().unwrap())).collect::<String>();

    let body_bytes = body::to_bytes(Body::from(body), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    let final_request = format!("{}{}{}\r\n", request_line, headers, body_str);
    println!("REQUEST BODY: {}", final_request);

    println!("REQUESTED ENDPOINT: {}", endpoint);


    let bro = state.map.get(endpoint).unwrap().value().clone();
    let id = uuid7();
  //  bro.send_main(ProtocolMessage::Open(id.clone())).await.unwrap();
    bro.send_main(ProtocolMessage::Message(WebsocketMessage { 
        id: id.clone(),
        code: ControlCode::Open,
        data: final_request.as_bytes().to_vec()

    })).boxed().await.unwrap();


    let chan = bro.get_channel(&id).await;
    let mut resp = Vec::new();
    loop {
        if let Ok(msg) = chan.recv().await {
            if msg.code == ControlCode::Close {
                println!("SAW THE CONTROL CODE...");
                break;
            }
            println!("RECEIVING PACKET...");
            resp.extend_from_slice(&msg.data);
        } else {
            // TODO: Throw error.
            break
        }
    }
    println!("OUT OF LOOPPP");
    
    // loop {
    //     if let Ok(packet) = chan.recv().await {
    //         match packet {

    //         }
    //     }
    // }

    // let resp = bro.get_channel(&id).await.recv().await.unwrap();

    // println!("BRO {:#?}", resp);



    
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);
    response.parse(&resp).unwrap();


 


    let mut body: &[u8] = &[];
    for i in 0..(resp.len() - 3) {
        if resp[i..i+4] == [13, 10, 13, 10] {
            body = &resp[i+4..];
            break
        }
    }

    // println!("BRO: {:?}", from_utf8(body).unwrap().to_string());

    // TODO: Missing parts of request.
    
    let mut hyper_response = hyper::Response::builder()
        .status(response.code.unwrap());
    for header in response.headers {
        hyper_response = hyper_response.header(header.name, header.value);
    }
    let hyper_response = hyper_response.body(Body::from(body.to_vec())).unwrap();


    


    // hyper_response

    hyper_response

}

async fn check_availability(State(state): State<Arc<KubeportServer>>, Path(name): Path<String>) -> (StatusCode, String)  {
    (StatusCode::ACCEPTED, (!state.has_route(&name)).to_string())
}


async fn websocket_handler(Path(pathname): Path<String>, ws: WebSocketUpgrade, State(state): State<Arc<KubeportServer>>) -> Response {
    ws.on_upgrade(|socket| handle_websocket_connection(pathname, socket, state))
}

async fn handle_websocket_connection(stream: String, ws: WebSocket, state: Arc<KubeportServer>) {
    info!("Starting a reverse proxy for stream [{}]...", stream);
    state.map.insert(stream.clone(), WebsocketProxy::create(ws));
    info!("Started a reverse proxy for stream [{}]...", stream);
}


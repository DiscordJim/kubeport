use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::{body::Bytes, extract::{ws::WebSocket, Path, State, WebSocketUpgrade}, http::StatusCode, response::{IntoResponse, Response}, routing::{get, post}, Router};
use dashmap::DashMap;
use tokio::{io::AsyncReadExt, net::{TcpListener, TcpSocket, TcpStream}};

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
        .route("/:service", get(handle_ws))
        .route("/available/:name", post(check_availability))
        .route("/forward/:service", post(connect_to_route))
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

async fn handle_websocket_connection(ws: WebSocket) {
    println!("Got a websocket request.");
}

async fn connect_to_route(State(state): State<KubeportServer>, Path(service): Path<String>, body: Bytes) -> (StatusCode, String) {
    // println!("Waiting on bro");
    // let mut bro = state.listener.accept().await.unwrap().0;

    // let mut boof = [0u8; 4];
    // println!("Waiting on read");
    // let buf = bro.read(&mut boof).await.unwrap();


    // println!("Bro: {:?}", boof);

    println!("Hello: {:?}", body);

    let conn = TcpStream::connect("127.0.0.1:7969").await.unwrap();
    println!("CONNECTED");
    

    (StatusCode::ACCEPTED, "hello".to_string())
}


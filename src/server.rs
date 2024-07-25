use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Router};
use dashmap::DashMap;

#[derive(Clone)]
pub struct KubeRoute {

}


#[derive(Clone, Default)]
pub struct KubeportServer {
    map: DashMap<String, KubeRoute>
}

impl KubeportServer {
    pub async fn spin() {
        run_kubeport_server(KubeportServer::default()).await;
    }
    pub fn has_route(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }
}

pub async fn run_kubeport_server(server: KubeportServer) {

    let app = Router::new()
        .route("/", get(root))
        .route("/:service", get(connect_to_route))
        .route("/available/:name", post(check_availability))
        .route("/forward/:service", post(connect_to_route))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();


}

async fn check_availability(State(state): State<KubeportServer>, Path(name): Path<String>) -> (StatusCode, String)  {


    (StatusCode::ACCEPTED, (!state.has_route(&name)).to_string())
}

async fn connect_to_route(State(state): State<KubeportServer>, Path(name): Path<String>) -> (StatusCode, String) {
    (StatusCode::ACCEPTED, "hello".to_string())
}

async fn root() -> &'static str {
    "hello world"
}
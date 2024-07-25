use axum::{extract::{Path, State}, response::IntoResponse, routing::get, Router};



#[derive(Clone)]
pub struct KubeportServer {

}

impl KubeportServer {
    pub async fn spin() {
        run_kubeport_server(KubeportServer {}).await;
    }
    
}

pub async fn run_kubeport_server(server: KubeportServer) {

    let app = Router::new()
        .route("/", get(root))
        .route("/check/:name", get(check_availability))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();


}

async fn check_availability(State(state): State<KubeportServer>, Path(name): Path<String>) -> impl IntoResponse {
    
}

async fn root() -> &'static str {
    "hello world"
}
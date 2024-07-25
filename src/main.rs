use kubeport::server::KubeportServer;



#[tokio::main]
async fn main() {
    let port_range = 10000..=10_050;

    let server = KubeportServer::spin().await;

    println!("Hello, world!");
}

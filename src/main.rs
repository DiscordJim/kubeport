use anyhow::Result;
use clap::Parser;
use kubeport::{client::run_client, server::KubeportServer};


#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    command: Command
}


//rkyv = { version = "0.7.44", features = ["bytecheck", "validation"] }

#[derive(Parser, Debug, PartialEq)]
pub enum Command {
    Client,
    Server
}

#[tokio::main]
async fn run(command: Command) -> Result<()>{
    if command == Command::Client {
        run_client().await
    } else {
        KubeportServer::spin().await
    }
}


fn main() -> Result<()> {
    run(Args::parse().command)
}

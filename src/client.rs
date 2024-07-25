

use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;

use crate::commons::stitch_streams;




pub async fn run_client() -> Result<()> {


    let (ws_stream, _) = connect_async("ws://localhost:8000/name").await?;
    let ws_stream = ws_stream;

    let conn = TcpStream::connect("locahost:4032").await?;
    
    println!("Fully connected.");
    // stitch_streams(stream_a, stream_b).await;


    // loop {
    //     stitch_streams().await;
    // }
   
    Ok(())
    
}
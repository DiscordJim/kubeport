use std::sync::Mutex;

use flume::{Receiver, Sender};
use tokio::net::{tcp::{ReadHalf, WriteHalf}, TcpStream};








// pub struct TcpStreamProxy {
//     recv: Sender<Vec<u8>>,
//     send: Receiver<Vec<u8>>
// }

// impl TcpStreamProxy {
//     pub fn new(stream: TcpStream) -> Self {
//         let (read, write) = stream.into_split();
//     }
// }
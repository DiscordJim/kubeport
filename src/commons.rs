


use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use flume::{Receiver, Sender};
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::{tcp::{ReadHalf, WriteHalf}, TcpStream}};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use uuid7::Uuid;


pub struct WebsocketProxy {
    send: Sender<WebsocketMessage>,
    recv: Receiver<WebsocketMessage>
}

impl Clone for WebsocketProxy {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
            recv: self.recv.clone()
        }
    }
}




#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolMessage {
    Open(Uuid),
    Message(WebsocketMessage)

}

impl ProtocolMessage {
    pub async fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
}

impl WebsocketProxy {
    pub fn create(ws: WebSocket) -> Self {
        let (send, recv) = flume::bounded::<WebsocketMessage>(300);


        let (mut ws_sender, mut ws_receiver) = ws.split();

        tokio::spawn({
            let send = send.clone();
            async move {
                while let Some(Ok(Message::Binary(bin))) = ws_receiver.next().await {
                    send.send_async(WebsocketMessage {
                        id: Uuid::default(),
                        data: bin
                    }).await.unwrap();
                }
            }
        });

        tokio::spawn({
            let recv = recv.clone();
            async move {
                while let Ok(msg) = recv.recv_async().await {
                    ws_sender.send(Message::Binary(msg.data)).await.unwrap();
                }
            }
        });




        Self {
            send,
            recv
        }
    }

    pub async fn recv(&self) -> Result<Vec<u8>> {
        Ok(self.recv.recv_async().await.map(|c| c.data)?)
    }
    pub async fn send(&self, data: Vec<u8>) -> Result<()> {
        self.send.send_async(WebsocketMessage {
            id: Uuid::default(),
            data
        }).await?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebsocketMessage {
    pub id: Uuid,
    pub data: Vec<u8>
}



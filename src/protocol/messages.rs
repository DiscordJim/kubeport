use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};
use tokio_tungstenite::tungstenite::Message;
use uuid7::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolMessage {
    Establish(String),
    Open(Uuid),
    Message(WebsocketMessage),
    Control(ControlCode)
    // Close(Uuid)

}



impl ProtocolMessage {
    pub async fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
    pub async fn to_message(&self) -> Result<Message> {
        Ok(Message::Binary(self.serialize().await?))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum ControlCode {
    Neutral,
    Open,
    Close
}


#[derive(Serialize, Deserialize, Debug)]
pub struct WebsocketMessage {
    pub id: Uuid,
    pub code: ControlCode,
    pub data: Vec<u8>
}


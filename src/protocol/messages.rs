// use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Error, Result};
// use rkyv::{
//     ser::{serializers::AllocSerializer, Serializer}, AlignedVec, Archive, Deserialize, Infallible, Serialize
// };
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use uuid7::Uuid;

// #[derive(Archive, Deserialize, Serialize)]
// #[archive(check_bytes)]
#[derive(Serialize, Deserialize)]
pub enum ProtocolMessage {
    Establish(String),
    Open(u32),
    Message(WebsocketMessage),
    Control(ControlCode), // Close(Uuid)
}

impl ProtocolMessage {
    pub fn deserialize(bytes: &[u8]) -> Result<ProtocolMessage> {
        // let archived = rkyv::check_archived_root::<Self>(bytes)
        // .map_err(|e| anyhow!("Failed to deserialize a protocol packet with error: {e}"))?;
        
        // let obj : ProtocolMessage = archived.deserialize(&mut Infallible)?;

        Ok(bincode::deserialize(bytes)?)
    }
    pub fn serialize(&self) -> Result<Vec<u8>> {
        // let mut serializer = AllocSerializer::<0>::default();
        // serializer.serialize_value(self).unwrap();
        // let bytes = serializer.into_serializer().into_inner();

        Ok(bincode::serialize(&self)?.to_vec())
        // Ok(bincode::serialize(self)?)
    }
    pub async fn to_message(&self) -> Result<Message> {
        let bytes = self.serialize()?;
        Ok(Message::Binary(bytes))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
// #[derive(Archive, Deserialize, Serialize)]
// #[archive(check_bytes)]
pub enum ControlCode {
    Neutral,
    Open,
    Close,
}

#[derive(Serialize, Deserialize, Debug)]
// #[derive(Archive, Deserialize, Serialize)]
// #[archive(check_bytes)]
pub struct WebsocketMessage {
    pub id: u32,
    pub code: ControlCode,
    pub data: Vec<u8>,
}

use anyhow::{anyhow, Result};
use rkyv::Archive;
use rkyv::{
    ser::{serializers::AllocSerializer, Serializer},
    AlignedVec, Deserialize, Serialize,
};

#[derive(Archive, Deserialize, Serialize, PartialEq)]
#[archive(check_bytes, compare(PartialEq))]
pub enum ControlCode {
    Neutral,
    Open,
    Close,
}

#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
pub struct WebsocketMessage {
    pub id: u32,
    pub code: ControlCode,
    pub data: Vec<u8>,
}

#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
pub enum ProtocolMessage {
    Establish(u32),
    Open(u32),
    Message(WebsocketMessage)
}

impl ProtocolMessage {
    pub fn from_bytes(vector: &AlignedVec) -> Result<&ArchivedProtocolMessage> {
        Ok(rkyv::check_archived_root::<Self>(vector.as_ref())
            .map_err(|e| anyhow!("Deserialization failed. Error: {e}"))?)
    }
    pub fn to_bytes(&self) -> Result<AlignedVec> {
        let mut serializer = AllocSerializer::<0>::default();
        serializer.serialize_value(self)?;
        let bytes = serializer.into_serializer().into_inner();
        Ok(bytes)
    }
}



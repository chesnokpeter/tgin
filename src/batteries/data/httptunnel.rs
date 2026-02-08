use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;



#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelRequest {
    pub id: Uuid,
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TunnelResponse {
    pub id: Uuid,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}
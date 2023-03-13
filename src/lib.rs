use std::collections::HashMap;
use reqwest::{ClientBuilder, Client};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Internal request error.")]
    GetRequest(#[from] reqwest::Error),
    #[error("Failed to find bridge.")]
    Failed
}

#[derive(Error, Debug)]
pub enum PairError {
    #[error("Internal request error.")]
    PostRequest(#[from] reqwest::Error),
    #[error("Link button has not been pressed.")]
    LinkButtonNotPressed,
    #[error("Failed to find bridge.")]
    Failed
}

#[derive(Error, Debug)]
pub enum ListLightsError {
    #[error("Internal request error.")]
    GetRequest(#[from] reqwest::Error),
    #[error("Failed to find bridge.")]
    Failed
}

#[derive(Debug)]
pub struct HueBridge {
    username: String,
    ip_addr: String,
    client: Client
}

impl HueBridge {
    fn new(username: String, ip_addr: String) -> HueBridge {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("hue-application-key", username.parse().unwrap());

        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .expect("Unable to create reqwest client.");

        HueBridge { username, ip_addr, client }
    }

    fn build_req(&self, url: &str) -> String {
        format!("https://{}/api/{}{}", self.ip_addr, self.username, url)
    }

    pub async fn list_lights(&self) -> Result<Vec<Light>, ListLightsError> {
        let req = format!("https://{}/{}", self.ip_addr, "/clip/v2/resource/light");

        let lights: Vec<Light> = self.client.get(req)
            .send()
            .await?
            .json::<Map<String, Value>>()
            .await?
            .iter()
            .filter_map(|(xkey, xvalue)| {
                let name: String = xvalue.get("name")?.as_str()?.to_string();
                let product_name = xvalue.get("productname")?.as_str()?.to_string();
                let state_opt = xvalue.get("state")?.to_owned();
                let state: LightState = serde_json::from_value(state_opt).unwrap();
                Some(Light { 
                    id: xkey.to_owned(), 
                    name, 
                    product_name,
                    state
                })
            })
            .collect();

        Ok(lights)
    }

    pub async fn discover() -> Result::<String, DiscoveryError> {
        let bridge = reqwest::get("https://discovery.meethue.com/".to_string())
            .await?
            .json::<Vec<Map<String, Value>>>()
            .await?
            .into_iter()
            .next()
            .ok_or(DiscoveryError::Failed)?;
        
        let json_obj = &bridge.get("internalipaddress")
            .ok_or(DiscoveryError::Failed)?;

        match json_obj {
            Value::String(ip_addr) => Ok(ip_addr.to_owned()),
            _ => Err(DiscoveryError::Failed),
        }
    }

    pub async fn pair(bridge_ip: String) -> Result<HueBridge, PairError> {
        let req = format!("https://{}/api", bridge_ip);

        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;

        let response = client.post(req)
            .json(&serde_json::json!({ "devicetype": Uuid::new_v4().to_string() }))
            .send()
            .await?
            .json::<Vec<Map<String, Value>>>()
            .await?
            .into_iter()
            .next()
            .ok_or(PairError::Failed)?;

        let json_obj = response
            .get("success")
            .ok_or(PairError::LinkButtonNotPressed)?
            .get("username")
            .ok_or(PairError::Failed)?;

        match json_obj {
            Value::String(username) => Ok(
                HueBridge::new(
                    username.to_owned(),
                    bridge_ip
                )
            ),
            _ => Err(PairError::Failed)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Light {
    id: String,
    name: String,
    product_name: String,
    state: LightState
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightState {
    on: bool,
    bri: u32,
    hue: u32,
    sat: u32,
    ct: u32,
    #[serde(rename = "colormode")]
    color_mode: String
}

#[cfg(test)]
mod tests {
    use super::*;

    static BRIDGE_IP: &str = "10.0.99.56";
    static BRIDGE_KEY: &str = "9XkuVXXI4cxX9SpoCJosjbqvEUZncoX3TuvweAlS";

    #[tokio::test]
    async fn discover_bridge() {
        let result = HueBridge::discover().await;
        assert_eq!(true, result.is_ok());
        assert_eq!(BRIDGE_IP, result.unwrap());
    }

    // Paired button needs to be pressed
    #[tokio::test]
    async fn pair_bridge() {
        let result = HueBridge::pair(BRIDGE_IP.to_string()).await;
        assert_eq!(result.is_ok(), true);
    }

    #[tokio::test]
    async fn list_lights() {
        let bridge = HueBridge::new(
            BRIDGE_KEY.to_string(),
            BRIDGE_IP.to_string()
        );

        let result = bridge.list_lights().await;
        assert_eq!(result.is_ok(), true);
        if let Ok(lights) = result {
            println!("{:?}", lights);
        }
    }
}

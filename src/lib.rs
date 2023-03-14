use std::collections::HashMap;
use reqwest::{ ClientBuilder, Client };
use serde::{ Deserialize, Serialize };
use serde_json::{ Map, Value };
use uuid::Uuid;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HueError {
    #[error("Error sending request")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Error parsing response")]
    Parsing(#[from] serde_json::Error),
    #[error("Link button has not been pressed.")]
    LinkButtonNotPressed,
    #[error("Invalid json ({msg:?})")]
    InvalidData {
        msg: String
    }
}

#[derive(Debug)]
pub struct HueBridge {
    base_url: String,
    client: Client
}

impl HueBridge {
    fn new(username: String, bridge_ip: String) -> HueBridge {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("hue-application-key", username.parse().unwrap());

        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .expect("Unable to create reqwest client.");

        let base_url = format!("https://{}", bridge_ip);

        HueBridge { base_url, client }
    }

    pub async fn list_lights(&self) -> Result<Vec<Light>, HueError> {
        let req = format!("{}{}", self.base_url, "/clip/v2/resource/light");

        let json_response = self.client.get(req)
            .send()
            .await?
            .json::<Value>()
            .await?;

        let lights = serde_json::from_value::<HashMap<String, Value>>(json_response)?["data"]
            .as_array()
            .ok_or(HueError::InvalidData { msg: "couldn't parse list_lights data json object".to_string() })?
            .into_iter()
            .filter_map(|in_value| {
                if let Value::Object(x) = in_value {
                    Some(Light {
                        id: x["id"].to_string(), 
                        name: x["metadata"]["name"].to_string(),
                        is_on: x["on"]["on"].as_bool().unwrap(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(lights)
    }

    pub async fn discover() -> Result::<String, HueError> {
        let req = format!("https://discovery.meethue.com/");

        let response = reqwest::get(req)
            .await?
            .json::<Value>() // Gives back a json array of a single result
            .await?;

        let bridges = serde_json::from_value::<Vec<HashMap<String, Value>>>(response)?
            .into_iter()
            .next() // Check first item exists and move to parsing it
            .ok_or(HueError::InvalidData { msg: "didn't find any bridges".to_string() })?;

        let bridge = bridges["internalipaddress"].as_str()
            .ok_or(HueError::InvalidData { msg: "couldn't parse bridge ip address".to_string() })?
            .to_owned();
        
        Ok(bridge)
    }

    pub async fn pair(bridge_ip: String) -> Result<HueBridge, HueError> {
        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;

        let req = format!("https://{}/api", bridge_ip);

        let response = client.post(req)
            .json(&serde_json::json!({ "devicetype": Uuid::new_v4().to_string() }))
            .send()
            .await?
            .json::<Value>()
            .await?;
            
        let json_obj = serde_json::from_value::<Vec<Map<String, Value>>>(response)?
            .into_iter()
            .next()
            .ok_or(
                HueError::InvalidData { msg: "Failed to parse pair result.".to_string() }
            )?;
            
        if let Value::Object(succes_obj) = &json_obj["success"] {
            let username = succes_obj["username"].as_str()
                .ok_or(
                    HueError::InvalidData { msg: "Failed to parse username pair result.".to_string() }
                )?.to_string();

            Ok(HueBridge::new(username, bridge_ip))
        } else if json_obj["error"]["type"].as_i64().unwrap_or(0) == 101 {
            Err(HueError::LinkButtonNotPressed)
        } else {
            Err(HueError::InvalidData { msg: "Unknown error occured".to_string() })
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Light {
    id: String,
    name: String,
    is_on: bool
}

#[cfg(test)]
mod tests {
    use super::*;

    static BRIDGE_IP: &str = "10.0.99.56";
    static BRIDGE_KEY: &str = "9XkuVXXI4cxX9SpoCJosjbqvEUZncoX3TuvweAlS";

    #[tokio::test]
    async fn discover_bridge() {
        let result = HueBridge::discover().await;
        dbg!(&result);
        assert_eq!(true, result.is_ok());
        assert_eq!(BRIDGE_IP, result.unwrap());
    }

    // Paired button needs to be pressed
    #[tokio::test]
    async fn pair_bridge() {
        let result = HueBridge::pair(BRIDGE_IP.to_string()).await;
        dbg!(&result);
        assert_eq!(result.is_ok(), true);
    }

    #[tokio::test]
    async fn list_lights() {
        let bridge = HueBridge::new(
            BRIDGE_KEY.to_string(),
            BRIDGE_IP.to_string()
        );

        let result = bridge.list_lights().await;
        dbg!(&result);
        assert_eq!(result.is_ok(), true);
    }
}

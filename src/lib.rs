use std::collections::HashMap;
use reqwest::{ ClientBuilder, Client };
use serde_json::{ Map, Value, json };
use uuid::Uuid;
use thiserror::Error;

mod light;

#[derive(Error, Debug)]
pub enum HueError {
    #[error("Error sending request")]
    Request(#[from] reqwest::Error),
    #[error("Error parsing response")]
    Parsing(#[from] serde_json::Error),
    #[error("Link button has not been pressed.")]
    LinkButtonNotPressed,
    #[error("Invalid json ({msg:?})")]
    InvalidData {
        msg: String
    },
    #[error("Unable to create reqwest client.")]
    ClientCreate
}

#[derive(Debug)]
pub struct HueBridge {
    base_url: String,
    client: Client
}

impl HueBridge {
    fn new(username: String, bridge_ip: String) -> Result<HueBridge, HueError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("hue-application-key", username.parse().unwrap());

        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .map_err(|_| { HueError::ClientCreate })?;

        let base_url = format!("https://{}", bridge_ip);

        Ok(HueBridge { base_url, client })
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
            .ok_or(HueError::InvalidData { msg: "didn't find any bridges".into() })?;

        let bridge = bridges["internalipaddress"].as_str()
            .ok_or(HueError::InvalidData { msg: "couldn't parse bridge ip address".into() })?
            .to_owned();
        
        Ok(bridge)
    }

    pub async fn pair(bridge_ip: String) -> Result<HueBridge, HueError> {
        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;

        let req = format!("https://{}/api", bridge_ip);

        let response = client.post(req)
            .json(&json!({ "devicetype": Uuid::new_v4().to_string() }))
            .send()
            .await?
            .json::<Value>()
            .await?;
            
        let json_obj = serde_json::from_value::<Vec<Map<String, Value>>>(response)?
            .into_iter()
            .next()
            .ok_or(
                HueError::InvalidData { msg: "Failed to parse pair result.".into() }
            )?;
            
        if let Value::Object(succes_obj) = &json_obj["success"] {
            let username = succes_obj["username"]
                .as_str()
                .ok_or(HueError::InvalidData { msg: "Failed to parse username pair result.".to_string() })?.into();

            HueBridge::new(username, bridge_ip)
        } else if json_obj["error"]["type"].as_i64().unwrap_or(0) == 101 {
            Err(HueError::LinkButtonNotPressed)
        } else {
            Err(HueError::InvalidData { msg: "Unknown error occured".into() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use light::{ Color, Light };

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
        let bridge = HueBridge::new(BRIDGE_KEY.into(), BRIDGE_IP.into()).unwrap();

        let result = Light::list_lights(&bridge).await;
        assert_eq!(result.is_ok(), true);
    }

    #[tokio::test]
    async fn toggle_light() {
        let bridge = HueBridge::new(BRIDGE_KEY.into(), BRIDGE_IP.into()).unwrap();
        let lights = Light::list_lights(&bridge).await.unwrap();
    
        for light in lights {
            let result = light.toggle_power()
            //let result = light.change_color(Some(Color(0.4005, 0.2255)), None)
            //let result = light.change_color(None, Some(100.0))
                .on(&bridge)
                .await;
            assert_eq!(result.is_ok(), true);
        }
    }
}

use std::collections::HashMap;
use reqwest::{ ClientBuilder, Client };
use serde_json::{ Map, Value };
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

            HueBridge::new(username, bridge_ip)
        } else if json_obj["error"]["type"].as_i64().unwrap_or(0) == 101 {
            Err(HueError::LinkButtonNotPressed)
        } else {
            Err(HueError::InvalidData { msg: "Unknown error occured".to_string() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use light::Light;

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
        ).unwrap();

        let result = Light::list_lights(&bridge).await;
        dbg!(&result);
        assert_eq!(result.is_ok(), true);
    }

    #[tokio::test]
    async fn toggle_light() {
        let bridge = HueBridge::new(
            BRIDGE_KEY.to_string(),
            BRIDGE_IP.to_string()
        ).unwrap();

        let lights_res = Light::list_lights(&bridge).await;
        let lights = lights_res.as_ref().unwrap();
        let light = lights.first().unwrap();
    
        //let result = light.toggle_power()
        //let result = light.change_color(Some(Color(0.4005, 0.2255)), None)
        let result = light.change_color(None, Some(100.0))
            .on(&bridge)
            .await;
        dbg!(&result);
        assert_eq!(result.is_ok(), true);
    }
}

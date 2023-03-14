use std::collections::HashMap;
use reqwest::{ ClientBuilder, Client };
use serde::Serialize;
use serde_json::{ Map, Value, json };
use uuid::Uuid;
use thiserror::Error;

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

    pub async fn execute(&self, light_transaction: Result<LightTransaction, HueError>) -> Result<Value, HueError> {
        light_transaction?.execute(&self.client, self.base_url.clone()).await
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
            .ok_or(HueError::InvalidData { msg: "couldn't parse list_lights data json object".into() })?
            .into_iter()
            .filter_map(|in_value| {
                if let Value::Object(x) = in_value {
                    Some(Light {
                        id: x["id"].as_str()?.into(), 
                        name: x["metadata"]["name"].as_str()?.into(),
                        is_on: x["on"]["on"].as_bool()?,
                        brightness: x["dimming"]["brightness"].as_f64()?,
                        min_brightness: x["dimming"]["min_dim_level"].as_f64()?,
                        color: Color(
                            x["color"]["xy"]["x"].as_f64()?,
                            x["color"]["xy"]["y"].as_f64()?
                        )
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

#[derive(Debug)]
pub struct Light {
    id: String,
    name: String,
    is_on: bool,
    brightness: f64,
    min_brightness: f64,
    color: Color
}
#[derive(Debug, Clone, Serialize)]
pub struct Color(f64, f64);

#[derive(Debug)]
pub struct LightTransaction {
    light_id: String,
    body: Value
}

impl LightTransaction {
    async fn execute(&self, client: &Client, base_url: String) -> Result<Value, HueError> {
        let mut req = base_url;
        req.push_str("/clip/v2/resource/light/");
        req.push_str(&self.light_id);

        dbg!(&req);
        dbg!(&self.body);

        let result = client.put(req)
            .json(&self.body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        dbg!(&result);
        Ok(result)
    }
}

impl Light {
    fn toggle_state(&self) -> Result<LightTransaction, HueError> {
        let body = json!({ "on": { "on": !self.is_on } });
        Ok(LightTransaction { light_id: self.id.clone(), body })
    }

    fn change_color(&self, color: Option<Color>, brightness: Option<f64>) -> Result<LightTransaction, HueError> {

        let Color(x, y) = color.unwrap_or(self.color.clone());

        let body = json!({ 
            "dimming": { 
                "brightness": brightness.unwrap_or(50.0), 
            },
            "color": {
                "xy": {
                    "x": x,
                    "y": y
                }
            } 
        });
        dbg!(&body);
        Ok(LightTransaction { light_id: self.id.clone(), body })
    }
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
        ).unwrap();

        let result = bridge.list_lights().await;
        dbg!(&result);
        assert_eq!(result.is_ok(), true);
    }

    #[tokio::test]
    async fn toggle_light() {
        let bridge = HueBridge::new(
            BRIDGE_KEY.to_string(),
            BRIDGE_IP.to_string()
        ).unwrap();

        let lights_res = &bridge.list_lights().await;
        let lights = lights_res.as_ref().unwrap();
        let light = lights.first().unwrap();
    
        //let result = bridge.execute(light.toggle_state()).await;
        //let result = bridge.execute(light.change_color(Some(Color(0.4605, 0.2255)), None)).await;
        let result = bridge.execute(light.change_color(None, Some(50.0))).await;
        dbg!(&result);
        assert_eq!(result.is_ok(), true);
    }
}

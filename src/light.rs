use serde::Serialize;
use serde_json::{json, Value};

use crate::{HueError, HueBridge};

#[derive(Debug, Clone, Serialize)]
pub struct Color(pub f64, pub f64);

#[allow(dead_code)]
#[derive(Debug)]
pub struct Light {
    #[allow(unused_assignments)]
    id: String,
    name: String,
    is_on: bool,
    brightness: f64,
    min_brightness: f64,
    color: Color
}

#[allow(dead_code)]
impl Light {
    pub async fn list_lights(bridge: &HueBridge) -> Result<Vec<Light>, HueError> {
        let req = format!("{}{}", bridge.base_url, "/clip/v2/resource/light");

        let json_response = bridge.client.get(req)
            .send()
            .await?
            .json::<Value>()
            .await?;

        let lights = serde_json::from_value::<serde_json::Map<String, Value>>(json_response)?["data"]
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

    pub fn toggle_power(&self) -> LightTransaction {
        Light::toggle_power_id(self.id.clone(), !self.is_on)
    }

    pub fn toggle_power_id(id: String, is_on: bool) -> LightTransaction {
        LightTransaction { 
            light_id: id,
            body: json!({ "on": { "on": is_on } })
        }
    }

    pub fn change_color(&self, color: Option<Color>, brightness: Option<f64>) -> LightTransaction {
        Light::change_color_id(self.id.clone(), color, brightness, Some(self.min_brightness))
    }

    pub fn change_color_id(light_id: String, color: Option<Color>, brightness: Option<f64>, min_brightness: Option<f64>) -> LightTransaction {
        let mut body = serde_json::Map::new();

        if let Some(Color(x, y)) = color {
            let valid_color = 0.0..=1.0;
            if valid_color.contains(&x) && valid_color.contains(&y) {
                body.insert("color".into(), json!({ "xy": { "x": x, "y": y }}));
            }
        }

        if let Some(bri) = brightness {
            if (min_brightness.unwrap_or(2.0)..=100.0).contains(&bri) {
                body.insert("dimming".into(), json!({ "brightness": bri }));
            }
        }

        LightTransaction { 
            light_id,
            body: body.into()
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct LightTransaction {
    light_id: String,
    body: Value
}

#[allow(dead_code)]
impl LightTransaction {
    pub async fn on(&self, bridge: &HueBridge) -> Result<(), HueError> {
        let mut req = bridge.base_url.clone();
        req.push_str("/clip/v2/resource/light/");
        req.push_str(&self.light_id);

        bridge.client
            .put(req)
            .json(&self.body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(())
    }
}

use serde::Serialize;
use serde_json::{json, Value};

use crate::{HueError, HueBridge};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ColorXY { pub x: f64, pub y: f64, pub bri: f64 }

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ColorRGB { pub r: f64, pub g: f64, pub b: f64 }

pub enum Color {
    XY(ColorXY),
    RGB(ColorRGB),
}

impl ColorRGB {
    #[allow(non_snake_case)]
    pub fn as_xy(&self) -> ColorXY {
        // Source of math https://github.com/PhilipsHue/PhilipsHueSDK-iOS-OSX/commit/f41091cf671e13fe8c32fcced12604cd31cceaf3
        // Gamma correction
        let red = if self.r > 0.04045 { ((self.r + 0.055) / (1.0 + 0.055)).powf(2.4) } else { self.r / 12.92 };
        let green = if self.g > 0.04045 { ((self.g + 0.055) / (1.0 + 0.055)).powf(2.4) } else { self.g / 12.92 };
        let blue = if self.b > 0.04045 { ((self.b + 0.055) / (1.0 + 0.055)).powf(2.4) } else { self.b / 12.92 };

        // Convert to XYZ
        let X = red * 0.649926 + green * 0.103455 + blue * 0.197109;
        let Y = red * 0.234327 + green * 0.743075 + blue * 0.022598;
        let Z = red * 0.0000000 + green * 0.053077 + blue * 1.035763;

        // Convert to XY
        let x = X / (X + Y + Z);
        let y = Y / (X + Y + Z);
        let bri = Y * 254.0;
        ColorXY { x, y, bri }
    }
}

impl ColorXY {
    pub fn new(rgb: &[f64; 3]) -> Self {
        ColorRGB { r: rgb[0].into(), g: rgb[1].into(), b: rgb[2].into() }.as_xy()
    }

    #[allow(non_snake_case)]
    pub fn as_rgb(&self) -> ColorRGB {
        let x = self.x; // the given x values
        let y = self.y; // the given y value
        let z = 1.0 - x - y;

        let Y = self.bri; // The given brightness value
        let X = (Y / y) * x;
        let Z = (Y / y) * z;

        // Convert to RGB using Wide RGB D65 conversion (THIS IS A D50 conversion currently)
        let mut r = X  * 1.4628067 - Y * 0.1840623 - Z * 0.2743606;
        let mut g = -X * 0.5217933 + Y * 1.4472381 + Z * 0.0677227;
        let mut b = X  * 0.0349342 - Y * 0.0968930 + Z * 1.2884099;

        // Apply reverse gamma correction
        r = if r <= 0.0031308 { 12.92 * r } else { (1.0 + 0.055) * r.powf(1.0 / 2.4) - 0.055 };
        g = if g <= 0.0031308 { 12.92 * g } else { (1.0 + 0.055) * g.powf(1.0 / 2.4) - 0.055 };
        b = if b <= 0.0031308 { 12.92 * b } else { (1.0 + 0.055) * b.powf(1.0 / 2.4) - 0.055 };
        ColorRGB { r, g, b }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Light {
    #[allow(unused_assignments)]
    pub id: String,
    pub name: String,
    pub is_on: bool,
    pub brightness: f64,
    pub min_brightness: f64,
    pub color: ColorXY,
}

#[allow(dead_code)]
impl Light {
    pub async fn list_lights(bridge: &HueBridge) -> Result<Vec<Light>, HueError> {
        let req = format!("https://{}{}", bridge.bridge_ip, "/clip/v2/resource/light");

        let client = bridge.build_client()?;
        let json_response = client.get(req)
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
                    let is_on = x["on"]["on"].as_bool()?;
                    let brightness = x["dimming"]["brightness"].as_f64()?;
                    let color_x = x["color"]["xy"]["x"].as_f64()?;
                    let color_y = x["color"]["xy"]["y"].as_f64()?;
                    Some(Light {
                        id: x["id"].as_str()?.into(), 
                        name: x["metadata"]["name"].as_str()?.into(),
                        is_on,
                        brightness,
                        min_brightness: x["dimming"]["min_dim_level"].as_f64()?,
                        color: ColorXY {
                            x: color_x,
                            y: color_y,
                            bri: brightness
                        },
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

        if let Some(color) = color {
            let color_xy = match color {
                Color::XY(color) => { color },
                Color::RGB(color) => { color.as_xy() },
            };
            let valid_color = 0.0..=1.0;
            if valid_color.contains(&color_xy.x) && valid_color.contains(&color_xy.y) {
                body.insert("color".into(), json!({ "xy": { "x": color_xy.x, "y": color_xy.y }}));
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
        let mut req = format!("https://{}", bridge.bridge_ip);
        req.push_str("/clip/v2/resource/light/");
        req.push_str(&self.light_id);

        let client = bridge.build_client()?;
        client
            .put(req)
            .json(&self.body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(())
    }
}
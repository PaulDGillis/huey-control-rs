use std::collections::HashMap;
use std::error::Error;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct HueBridge {
    id: String,
    internalipaddress: String,
    port: u32
}

impl HueBridge {
    fn build_req(&self, url: String) -> String {
        String::from(format!("https://{}{}", self.internalipaddress, url))
    }

    pub fn discover() -> Result::<String, ()> {
        let bridges = reqwest::blocking::get(String::from("https://discovery.meethue.com/"))
            .expect("")
            .json::<Vec::<Map<String, serde_json::Value>>>()
            .expect("");

        let key = String::from("internalipaddress");

        let test = bridges[0]["internalipaddress"];
        if let serde_json::Value::String(tes) = test {
            return Ok(tes);
        }

        Result::Err(())
    }

    pub fn pair(&self, client: Client) -> Result<(), reqwest::Error> {
        let mut post_body = HashMap::new();
        post_body.insert(
            "devicetype", 
            format!("rs_hue_app#{}", Uuid::new_v4().to_string())
        );

        let req = self.build_req(String::from("/api/"));

        client.post(req)
            .json(&post_body)
            .send()
            .map(|_| { () })
    }
}
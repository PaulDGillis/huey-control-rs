use std::collections::HashMap;
use std::error::Error;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;

#[derive(Debug)]
pub struct HueBridge {
    username: String,
    internalipaddress: String,
}

impl HueBridge {
    fn build_req(&self, url: String) -> String {
        String::from(format!("https://{}/api/{}{}", self.internalipaddress, self.username, url))
    }

    pub fn discover() -> Result::<String, ()> {
        let bridges = reqwest::blocking::get(String::from("https://discovery.meethue.com/"))
            .expect("")
            .json::<Vec::<Map<String, serde_json::Value>>>()
            .expect("");

        let test = &bridges[0]["internalipaddress"];
        if let serde_json::Value::String(tes) = test {
            return Ok(tes.to_string());
        }

        Result::Err(())
    }

    pub fn pair(client: Client, bridge_ip: String) -> Result<HueBridge, ()> {
        let uuid = Uuid::new_v4();

        let mut post_body = HashMap::new();
        post_body.insert(
            "devicetype", 
            format!("rs_hue_app#{}", uuid.to_string())
        );

        let req = String::from(format!("https://{}/api", bridge_ip));

        let response = client.post(req)
            .json(&post_body)
            .send()
            .expect("Failed to make request")
            .json::<Vec::<Map<String, serde_json::Value>>>()
            .expect("Failed to read response");

        if response.len() > 0 && response[0].contains_key("success") {
            if let serde_json::Value::String(username) = &response[0]["success"] {
                return Ok(
                    HueBridge { username: username.to_string(), internalipaddress: bridge_ip }
                );
            } else {
                println!("Invalid response")
            }
            
        } else {
            println!("Responses either 0 or doesn't succeed")
        }

        Result::Err(())
    }

    pub fn list_lights(&self) {
        let req = self.build_req(String::from("/lights/"));

        let bridges = reqwest::blocking::get(req)
            .expect("")
            .text()
            //.json::<Vec::<Map<String, serde_json::Value>>>()
            .expect("");

        println!("{} = ", bridges)
    }
}
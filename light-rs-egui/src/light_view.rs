use std::sync::Arc;

use eframe::egui::Widget;
use light_rs_core::{light::Light, HueBridge, HueError};
use poll_promise::Promise;

struct LightView {
    light_id: Arc<String>,
    name: String,
    is_on: bool,
    bridge: Arc<(String, String)>
}

impl LightView {
    pub fn new(light: &Light, bridge: &HueBridge) -> Self {
        Self {
            light_id: Arc::new(light.id.clone()),
            name: light.name.clone(),
            is_on: light.is_on,
            bridge: Arc::new((bridge.username.clone(), bridge.bridge_ip.clone()))
        }
    }

    fn toggle_light(&self) -> Promise<Result<(), HueError>> {
        let light_id = self.light_id.clone();
        let is_on = self.is_on;
        let bridge = self.bridge.clone();
        Promise::spawn_async(async move {
            let bridge = HueBridge::new(bridge.0.to_string(), bridge.1.to_string());
            Light::toggle_power_id(light_id.to_string(), !is_on).on(&bridge).await
        })
    }
}

impl Widget for LightView {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            ui.label(self.name);

            if ui.button("toggle").clicked() {
                self.toggle_light();
            }
        }).response
    }
}
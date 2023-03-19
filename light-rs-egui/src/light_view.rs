use light_rs_core::{light::Light, HueBridge};
use poll_promise::Promise;

use crate::toggle_switch::toggle_ui;

pub fn ui(light: &mut Light, bridge: &HueBridge, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
    ui.horizontal(|ui| {
        let is_on = light.toggle_promise.ready()
            .unwrap_or(&Some(light.is_on))
            .unwrap_or(light.is_on);
        ui.label(&light.name);

        if toggle_ui(ui, is_on).clicked() {
            let light_id = light.id.clone();
            let bridge = bridge.clone();

            let next_is_on = !is_on;
            light.is_on = next_is_on;

            light.toggle_promise = Promise::spawn_async(async move { 
                Light::toggle_power_id(light_id.to_string(), next_is_on)
                    .on(&bridge)
                    .await
                    .map(|_| { next_is_on })
                    .ok()
            });
        }
    }).response
}
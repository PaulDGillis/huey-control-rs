use light_rs_core::{light::Light, HueBridge};
use poll_promise::Promise;

use crate::toggle_switch::toggle_ui;

fn toggle_light(light: &mut Light, bridge: &HueBridge) -> Promise<Option<bool>> {
    let light_id = light.id.clone();
    let next_is_on = !light.is_on;
    let bridge = bridge.clone();
    light.is_on = next_is_on;

    Promise::spawn_async(async move { 
        Light::toggle_power_id(light_id.to_string(), next_is_on).on(&bridge).await.map(|_| { next_is_on }).ok()
    })
}

pub fn ui(light: &mut Light, bridge: &HueBridge, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
    ui.horizontal(|ui| {
        let mut is_on = false;
        if let Some(Some(is_on_rf)) = light.toggle_promise.ready() {
            is_on = is_on_rf.clone();
        }
        ui.label(&light.name);

        if toggle_ui(ui, is_on).clicked() {
            light.toggle_promise = toggle_light(light, bridge);
        }
    }).response
}
use eframe::egui::color_picker::color_edit_button_rgb;
use light_rs_core::light::{Color, ColorRGB};
use light_rs_core::{light::Light, HueBridge};
use poll_promise::Promise;

use crate::toggle_switch::toggle_ui;
use crate::egui::Slider;
use crate::Result;

pub struct LightsViewModel(Option<Promise<Result<Vec<LightViewModel>>>>);

impl LightsViewModel {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn ui(&mut self, bridge: &HueBridge, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        // Get or create async request to get lights from bridge
        let lights_promise = self.0.get_or_insert_with(|| {
            let bridge = bridge.clone();
            Promise::spawn_async(async move {
                Light::list_lights(bridge).await.map(|lights| {
                    lights.into_iter().map(|light| {
                        LightViewModel::new(light)
                    }).collect()
                })
            })
        });

        // Display ui spinner if promise isn't ready
        let Some(Ok(lights)) = lights_promise.ready_mut() else {
            return ui.spinner();
        };

        // Draw ui for the list of available lights
        ui.vertical(|ui| {
            lights.iter_mut().for_each(|viewmodel| {
                viewmodel.ui(bridge, ui);
            });    
        }).response
    }

}

pub struct LightViewModel {
    light: Light,
    toggle_promise: Option<Promise<Option<bool>>>,
    brightness_promise: Option<Promise<Option<()>>>,
    color_promise: Option<Promise<Option<()>>>,
    brightness: f64,
    color: [f32; 3]
}

impl LightViewModel {
    pub fn new(light: Light) -> Self {
        let brightness = light.brightness;
        let color_rgb = light.color.as_rgb();
        Self {
            light,
            toggle_promise: None,
            brightness_promise: None,
            color_promise: None,
            brightness,
            // TODO check f32 conversion
            color: [color_rgb.r as f32, color_rgb.g as f32, color_rgb.b as f32]
        }
    }

    pub fn ui(&mut self, bridge: &HueBridge, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            // Draw ui for nice toggle switch
            let is_on = if let Some(promise) = &self.toggle_promise {
                if let Some(Some(is_on)) = promise.ready() { is_on } else { &self.light.is_on }
            } else { &self.light.is_on };
    
            // Has toggle switch state changed?
            if toggle_ui(ui, *is_on).clicked() {
                // Execute change for local light struct
                let next_is_on = !is_on;
                self.light.is_on = next_is_on;

                // Create async request to change on bridge
                // TODO these state change promises should change back ui if request fails
                let light_id = self.light.id.clone();
                let bridge = bridge.clone();
                self.toggle_promise = Some(Promise::spawn_async(async move { 
                    Light::toggle_power_id(light_id, next_is_on)
                        .on(&bridge)
                        .await
                        .map(|_| { next_is_on })
                        .ok()
                }));
            }

            // Draw ui for color picker
            color_edit_button_rgb(ui, &mut self.color);

            // Convert ui color to Hue compatible CIE XY color
            let color = ColorRGB { r: self.color[0].into(), g: self.color[1].into(), b: self.color[2].into() }.as_xy();

            // Has color changed?
            if self.light.color.x != color.x && self.light.color.y != color.y {
                // Cancel previous request
                if self.color_promise.is_some() {
                    self.color_promise.take().unwrap().abort();
                    self.color_promise = None;
                }

                // Execute change for local light struct
                self.light.color = color;
                self.light.brightness = color.bri;

                // Create async request to change on bridge
                // TODO these state change promises should change back ui if request fails
                let light_id = self.light.id.clone();
                let bridge = bridge.clone();
                let color = color.clone();
                self.color_promise = Some(Promise::spawn_async(async move { 
                    Light::change_color_id(light_id.to_string(), Some(Color::XY(color)), Some(color.bri), None)
                        .on(&bridge)
                        .await
                        .ok()
                }));
            }

            // Draw ui for brightness slider
            ui.add(
                Slider::new(&mut self.brightness, self.light.min_brightness..=100.0)
                    .step_by(10.0)
                    .show_value(false)
            );

            // Has brightness changed?
            if self.brightness != self.light.brightness {
                // Cancel previous request
                if self.brightness_promise.is_some() {
                    self.brightness_promise.take().unwrap().abort();
                    self.brightness_promise = None;
                }

                // Execute change for local light struct
                let brightness = self.brightness;
                self.light.brightness = brightness;

                // Create async request to change on bridge
                // TODO these state change promises should change back ui if request fails
                let light_id = self.light.id.clone();
                let bridge = bridge.clone();
                self.brightness_promise = Some(Promise::spawn_async(async move { 
                    Light::change_color_id(light_id.to_string(), None, Some(brightness), None)
                        .on(&bridge)
                        .await
                        .ok()
                }));
            }

            // Draw ui for light name
            ui.label(&self.light.name);
        }).response
    }
}
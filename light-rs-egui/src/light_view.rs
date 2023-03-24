use eframe::egui::color_picker::color_edit_button_rgb;
use light_rs_core::light::{Color, ColorRGB, ColorXY};
use light_rs_core::{light::Light, HueBridge, HueError};
use poll_promise::Promise;

use crate::toggle_switch::toggle_ui;
use crate::egui::Slider;

pub struct LightsViewModel {
    lights: Option<Promise<Result<Vec<LightViewModel>, HueError>>>
}

pub struct LightViewModel {
    light: Light,
    toggle_promise: Option<Promise<Option<bool>>>,
    brightness_promise: Option<Promise<Option<()>>>,
    color_promise: Option<Promise<Option<()>>>,
    brightness: f64,
    color: [f32; 3]
}

impl LightsViewModel {
    pub fn new() -> Self {
        Self { lights: None }
    }

    pub fn ui(&mut self, bridge: &HueBridge, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let lights_promise = self.lights.get_or_insert_with(|| {
            let bridge = bridge.clone();
            Promise::spawn_async(async move {
                Light::list_lights(bridge).await
                    .map(|items| {
                        items.into_iter().map(|item| {
                            let brightness = item.brightness;
                            let color_rgb = ColorXY { x: item.color.x, y: item.color.y, bri: item.brightness }.as_rgb();
                            LightViewModel { light: item, toggle_promise: None, brightness_promise: None, color_promise: None, brightness, color: [color_rgb.r as f32, color_rgb.g as f32, color_rgb.b as f32] }
                        })
                        .collect()
                    })
            })
        });

        if let Some(Ok(lights)) = lights_promise.ready_mut() {
            ui.vertical(|ui| {
                lights.iter_mut().for_each(|viewmodel| {

                    ui.horizontal(|ui| {
                        let light = &mut viewmodel.light;    

                        // UI for power toggle
                        let is_on = if let Some(promise) = &viewmodel.toggle_promise {
                            if let Some(Some(is_on)) = promise.ready() { is_on } else { &light.is_on }
                        } else { &light.is_on };
                
                        if toggle_ui(ui, *is_on).clicked() {
                            let light_id = light.id.clone();
                            let bridge = bridge.clone();
                
                            let next_is_on = !is_on;
                            light.is_on = next_is_on;
                
                            viewmodel.toggle_promise = Some(Promise::spawn_async(async move { 
                                Light::toggle_power_id(light_id.to_string(), next_is_on)
                                    .on(&bridge)
                                    .await
                                    .map(|_| { next_is_on })
                                    .ok()
                            }));
                        }

                        // UI for color picker
                        color_edit_button_rgb(ui, &mut viewmodel.color);

                        let color = ColorRGB { r: viewmodel.color[0].into(), g: viewmodel.color[1].into(), b: viewmodel.color[2].into() }.as_xy();

                        if light.color.x != color.x && light.color.y != color.y {
                            // Change?
                            // Cancel previous request
                            if viewmodel.color_promise.is_some() {
                                viewmodel.color_promise.take().unwrap().abort();
                                viewmodel.color_promise = None;
                            }

                            let light_id = light.id.clone();
                            let bridge = bridge.clone();
                            light.color = color;
                            light.brightness = color.bri;
                            let color = color.clone();

                            viewmodel.color_promise = Some(Promise::spawn_async(async move { 
                                Light::change_color_id(light_id.to_string(), Some(Color::XY(color)), Some(color.bri), None)
                                    .on(&bridge)
                                    .await
                                    .ok()
                            }));
                        }

                        // UI for brightness slider
                        let light_min_brightness = light.min_brightness;
                        ui.add(
                            Slider::new(&mut viewmodel.brightness, light_min_brightness..=100.0)
                                .step_by(10.0)
                                .show_value(false)
                        );

                        if viewmodel.brightness != light.brightness {
                            // Cancel previous request
                            if viewmodel.brightness_promise.is_some() {
                                viewmodel.brightness_promise.take().unwrap().abort();
                                viewmodel.brightness_promise = None;
                            }

                            let light_id = light.id.clone();
                            let bridge = bridge.clone();
                            let brightness = viewmodel.brightness;
                            light.brightness = brightness;

                            viewmodel.brightness_promise = Some(Promise::spawn_async(async move { 
                                Light::change_color_id(light_id.to_string(), None, Some(brightness), None)
                                    .on(&bridge)
                                    .await
                                    .ok()
                            }));
                        }

                        // UI for light name
                        ui.label(&light.name);
                    });
                });    
            }).response
        } else {
            ui.spinner()
        }
    }

}
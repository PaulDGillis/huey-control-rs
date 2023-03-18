#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, Storage};
use light_rs_core::{HueError, HueBridge, light::Light};
use poll_promise::Promise;

mod light_view;

#[tokio::main]
async fn main() { // -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Light-rs",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    );
}

struct MyApp {
    bridge_ip: Option<Promise<Result<String, HueError>>>,
    bridge_key: Option<Promise<Result<HueBridge, HueError>>>,
    lights: Option<Promise<Result<Vec<Light>, HueError>>>,
}

const BRIDGE_IP_KEY: &'static str = "bridge_ip";
const BRIDGE_KEY_KEY: &'static str = "bridge_key";

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> MyApp {
        if let Some(store) = _cc.storage {
            if let Some(bridge_ip) = store.get_string(BRIDGE_IP_KEY) {
                let bridge_ip_promise: Promise<Result<String, HueError>> = Promise::from_ready(Ok(bridge_ip.clone()));

                if let Some(bridge_key) = store.get_string(BRIDGE_KEY_KEY) {
                    let bridge = HueBridge::new(bridge_key, bridge_ip.clone());
                    let bridge_promise: Promise<Result<HueBridge, HueError>> = Promise::from_ready(Ok(bridge));                    

                    return MyApp {
                        bridge_ip: Some(bridge_ip_promise),
                        bridge_key: Some(bridge_promise),
                        lights: None
                    };
                }
            }
        }

        MyApp {
            bridge_ip: None,
            bridge_key: None,
            lights: None
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let bridge_ip = self.bridge_ip.get_or_insert_with(|| {
            Promise::spawn_async(async move {
                HueBridge::discover().await
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match bridge_ip.ready() {
                Some(Ok(bridge_ip)) => {
                    let bridge = self.bridge_key.get_or_insert_with(|| {
                        let ip = bridge_ip.clone();
                        Promise::spawn_async(async move {
                            HueBridge::pair(ip).await
                        })
                    });

                    match bridge.ready() {
                        Some(Ok(bridge)) => {
                            let lights = self.lights.get_or_insert_with(|| {
                                let ip = bridge.bridge_ip.clone();
                                let username = bridge.username.clone();
                                Promise::spawn_async(async move {
                                    let bridge = HueBridge::new(username, ip);
                                    Light::list_lights(&bridge).await
                                })
                            });

                            match lights.ready() {
                                Some(Ok(lights)) => {
                                    ui.vertical(|ui| {
                                        lights.iter().for_each(|light| {
                                            ui.horizontal(|ui| {
                                                ui.label(light.name.clone());

                                                if ui.button("toggle").clicked() {
                                                    let light_id = light.id.clone();
                                                    let next = light.is_on;
                                                    let ip = bridge.bridge_ip.clone();
                                                    let username = bridge.username.clone();
                                                    Promise::spawn_async(async move {
                                                        let bridge = HueBridge::new(username, ip);
                                                        Light::toggle_power_id(light_id, !next).on(&bridge).await
                                                    });
                                                }
                                            });
                                        });
                                    });
                                },
                                _ => { ui.spinner(); }
                            }

                            if ui.button("Refresh").clicked() {
                                let ip = bridge.bridge_ip.clone();
                                let username = bridge.username.clone();
                                let lights_promise = Promise::spawn_async(async move {
                                    let bridge = HueBridge::new(username, ip);
                                    Light::list_lights(&bridge).await
                                });
                                self.lights = Some(lights_promise);
                            }
                        },
                        Some(Err(error)) => {
                            match error {
                                HueError::LinkButtonNotPressed => {
                                    let lab = ui.label("Link button not pressed.");
                                    if ui.button("Retry?").labelled_by(lab.id).clicked() {
                                        let ip = bridge_ip.clone();
                                        self.bridge_key = Some(
                                            Promise::spawn_async(async move {
                                                HueBridge::pair(ip).await
                                            })
                                        );
                                    }
                                },
                                _ => {}
                            }
                        },
                        _ => { ui.spinner(); }   
                    }
                }
                _ => { ui.spinner(); }
            }
        });
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Some(bridge_ip_promise) = &self.bridge_ip {
            if let Some(bridge_ip_result) = bridge_ip_promise.ready() {
                if let Some(bridge_ip) = bridge_ip_result.as_ref().ok() {
                    _storage.set_string(BRIDGE_IP_KEY, bridge_ip.into());
                }
            }
        }

        if let Some(bridge_promise) = &self.bridge_key {
            if let Some(bridge_result) = bridge_promise.ready() {
                if let Some(bridge) = bridge_result.as_ref().ok() {
                    _storage.set_string(BRIDGE_KEY_KEY, bridge.username.clone());
                }
            }
        }
    }
}

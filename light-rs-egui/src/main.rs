#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, Storage};
use light_rs_core::{HueError, HueBridge, light::Light};
use poll_promise::Promise;

mod light_view;
mod toggle_switch;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
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
    )
}

struct MyApp {
    bridge_ip: Promise<Result<String, HueError>>,
    bridge_key: Promise<Result<HueBridge, HueError>>,
    lights: Promise<Result<Vec<Light>, HueError>>,
}

const BRIDGE_IP_KEY: &'static str = "bridge_ip";
const BRIDGE_KEY_KEY: &'static str = "bridge_key";

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> MyApp {
        let mut init_bridge_ip = Err(HueError::Uninitialized);
        let mut init_bridge = Err(HueError::Uninitialized);
        if let Some(store) = _cc.storage {
            if let Some(bridge_ip) = store.get_string(BRIDGE_IP_KEY) {
                init_bridge_ip = Ok(bridge_ip.clone());
                if let Some(bridge_key) = store.get_string(BRIDGE_KEY_KEY) {
                    init_bridge = Ok(HueBridge::new(bridge_ip.clone(), bridge_key));
                }
            }
        }

        MyApp {
            bridge_ip: Promise::from_ready(init_bridge_ip),
            bridge_key: Promise::from_ready(init_bridge),
            lights: Promise::from_ready(Err(HueError::Uninitialized)),
        }
    }

    fn discover_bridge() -> Promise<Result<String, HueError>> {
        Promise::spawn_async(async move { HueBridge::discover().await })
    }

    fn pair_bridge(bridge_ip: &String) -> Promise<Result<HueBridge, HueError>> {
        let bridge_ip = bridge_ip.clone();
        Promise::spawn_async(async move { HueBridge::pair(bridge_ip).await })
    }

    fn list_lights(bridge: &HueBridge) -> Promise<Result<Vec<Light>, HueError>> {
        let bridge = bridge.clone();
        Promise::spawn_async(async move {
            let bridge = HueBridge::new(bridge.bridge_ip, bridge.username);
            Light::list_lights(&bridge).await
        })
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let bridge_ip = &self.bridge_ip;
            let bridge = &self.bridge_key;

            match bridge_ip.ready() {
                Some(Ok(bridge_ip)) => {
                    match bridge.ready() {
                        Some(Ok(bridge)) => {
                            match self.lights.ready_mut() {
                                Some(Ok(lights)) => {
                                    ui.vertical(|ui| {
                                        lights.iter_mut().for_each(|light| {
                                            light_view::ui(light, bridge, ui);
                                        });
                                    });
                                },
                                Some(Err(HueError::Uninitialized)) => { self.lights = Self::list_lights(bridge); },
                                _ => { ui.spinner(); }
                            }
                        },
                        Some(Err(HueError::Uninitialized)) => { self.bridge_key = Self::pair_bridge(bridge_ip); },
                        Some(Err(_)) => {
                            ui.vertical(|ui| {
                                ui.spinner();
                                if ui.button("Retry?").clicked() {
                                    self.bridge_key = Self::pair_bridge(bridge_ip);
                                }
                            });
                        },
                        _ => { ui.spinner(); }
                    }
                },
                Some(Err(HueError::Uninitialized)) => { self.bridge_ip = Self::discover_bridge(); },
                _ => { ui.spinner(); }
            }
        });
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Some(bridge_ip_res) = self.bridge_ip.ready() {
            if let Ok(bridge_ip) = bridge_ip_res {
                _storage.set_string(BRIDGE_IP_KEY, bridge_ip.into());
            }
        }

        if let Some(bridge_key_res) = self.bridge_key.ready() {
            if let Ok(bridge) = bridge_key_res {
                _storage.set_string(BRIDGE_KEY_KEY, bridge.username.clone());
            }
        }
    }
}

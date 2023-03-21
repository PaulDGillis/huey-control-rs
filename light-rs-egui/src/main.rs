#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::time::Duration;

use eframe::{egui, Storage, epaint::{Pos2, Vec2}};
use light_rs_core::{HueError, HueBridge};
use light_view::LightsViewModel;
use poll_promise::Promise;
use tray_icon::{TrayIconBuilder, TrayEvent, ClickEvent, Rectangle};

mod light_view;
mod toggle_switch;

const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 200.0;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        resizable: false,
        decorated: false,
        min_window_size: Some(Vec2::new(WIDTH, HEIGHT)),
        max_window_size: Some(Vec2::new(WIDTH, HEIGHT)),
        always_on_top: true,
        ..Default::default()
    };

    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("Light-rs - tray")
        .build()
        .unwrap();

    eframe::run_native(
        "Light-rs",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    )
}

struct MyApp {
    is_visible: bool,
    bridge_ip: Option<Promise<Result<String, HueError>>>,
    bridge: Option<Promise<Result<HueBridge, HueError>>>,
    light_viewmodel: LightsViewModel,
}

const BRIDGE_IP_KEY: &'static str = "bridge_ip";
const BRIDGE_KEY_KEY: &'static str = "bridge_key";

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> MyApp {
        let mut init_bridge_ip = None;
        let mut bridge = None;
        if let Some(store) = _cc.storage {
            if let Some(bridge_ip) = store.get_string(BRIDGE_IP_KEY) {
                init_bridge_ip = Some(Promise::from_ready(Ok(bridge_ip.clone())));
                if let Some(bridge_key) = store.get_string(BRIDGE_KEY_KEY) {
                    bridge = Some(
                        Promise::from_ready(
                            Ok(HueBridge::new(bridge_ip.clone(), bridge_key))
                        )
                    );
                }
            }
        }

        MyApp {
            is_visible: false,
            bridge_ip: init_bridge_ip,
            bridge,
            light_viewmodel: LightsViewModel::new(),
        }
    }

    fn pair_bridge(bridge_ip: &String) -> Promise<Result<HueBridge, HueError>> {
        let bridge_ip = bridge_ip.clone();
        Promise::spawn_async(async move { 
            HueBridge::pair(bridge_ip).await
        })
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(event) = TrayEvent::receiver().try_recv() {
            if event.event == ClickEvent::Left {
                let size = _frame.info().window_info.monitor_size.unwrap();
                let pos = Pos2::new((event.x as f32) - (WIDTH / 2.0), (size.y as f32) - 80.0 - HEIGHT - (HEIGHT / 2.0));
                _frame.set_window_pos(pos);
                
                let state = !self.is_visible;
                _frame.set_minimized(state);
                self.is_visible = state;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let bridge_ip = self.bridge_ip.get_or_insert_with(|| {
                Promise::spawn_async(async move { HueBridge::discover().await })
            });

            match bridge_ip.ready() {
                Some(Ok(bridge_ip)) => {
                    let bridge = self.bridge.get_or_insert_with(|| { Self::pair_bridge(bridge_ip) });
            
                    match bridge.ready_mut() {
                        Some(Ok(bridge)) => {
                            self.light_viewmodel.ui(bridge, ui);
                        },
                        Some(Err(_)) => {
                            ui.vertical(|ui| {
                                ui.spinner();
                                if ui.button("Retry?").clicked() {
                                    self.bridge = Some(Self::pair_bridge(bridge_ip));
                                }
                            });
                        },
                        _ => { ui.spinner(); }
                    }
                },
                _ => { ui.spinner(); }
            }
        });

        ctx.request_repaint_after(Duration::new(0, 100));
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Some(bridge_ip_opt) = &self.bridge_ip {
            if let Some(Ok(bridge_ip)) = bridge_ip_opt.ready() {
                _storage.set_string(BRIDGE_IP_KEY, bridge_ip.into());
            }
        }

        if let Some(bridge_key_opt) = &self.bridge {
            if let Some(Ok(bridge)) = bridge_key_opt.ready() {
                _storage.set_string(BRIDGE_KEY_KEY, bridge.username.clone());
            }
        }
    }
}

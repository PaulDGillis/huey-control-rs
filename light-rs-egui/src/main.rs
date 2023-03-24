#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crossbeam_channel::Receiver;
use eframe::{egui, Storage, epaint::{Pos2, Vec2}, IconData};
use light_rs_core::{HueError, HueBridge};
use light_view::LightsViewModel;
use poll_promise::Promise;
use tray_icon::{TrayIconBuilder, TrayEvent, ClickEvent, TrayIcon};

mod light_view;
mod toggle_switch;

const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 200.0;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png");
    let (icon_rgba, icon_width, icon_height) = load_icon(std::path::Path::new(path));

    let options = eframe::NativeOptions {
        resizable: false,
        decorated: false,
        transparent: true,
        min_window_size: Some(Vec2::new(WIDTH, HEIGHT)),
        max_window_size: Some(Vec2::new(WIDTH, HEIGHT)),
        always_on_top: true,
        run_and_return: false,
        icon_data: Some(IconData { rgba: icon_rgba.clone(), width: icon_width, height: icon_height }),
        ..Default::default()
    };

    let icon = tray_icon::icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon");

    let mut tray_icon = TrayIconBuilder::new()
        .with_tooltip("Light-rs - tray")
        .with_icon(icon);
    
    if cfg!(macos) {
        tray_icon = tray_icon.with_icon_as_template(true).with_menu_on_left_click(false);
    }

    eframe::run_native(
        "Light-rs",
        options,
        Box::new(move |cc| Box::new(MyApp::new(cc, tray_icon.build().unwrap()))),
    )
}

fn load_icon(path: &std::path::Path) -> (Vec<u8>, u32, u32) {
    let image = image::open(path)
        .expect("Failed to open icon path")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    (rgba, width, height)
}

struct MyApp {
    is_visible: bool,
    bridge_ip: Option<Promise<Result<String, HueError>>>,
    bridge: Option<Promise<Result<HueBridge, HueError>>>,
    light_viewmodel: LightsViewModel,
    tray_events: Receiver<TrayEvent>,
    _tray_icon: TrayIcon
}

const BRIDGE_IP_KEY: &'static str = "bridge_ip";
const BRIDGE_KEY_KEY: &'static str = "bridge_key";

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>, tray_icon: TrayIcon) -> MyApp {
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

        let (s, r) = crossbeam_channel::unbounded();

        let context = _cc.egui_ctx.clone();
        #[allow(unused_must_use)]
        TrayEvent::set_event_handler(Some(move |item| {
            s.send(item);
            context.request_repaint();
        }));

        MyApp {
            is_visible: false,
            bridge_ip: init_bridge_ip,
            bridge,
            light_viewmodel: LightsViewModel::new(),
            tray_events: r,
            _tray_icon: tray_icon
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
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Ok(event) = self.tray_events.try_recv() {
            if event.event == ClickEvent::Left {
                let state = !self.is_visible;

                self.is_visible = state;

                let x = {
                    if cfg!(windows) {
                        (((event.icon_rect.right - event.icon_rect.left)/2.0 + event.icon_rect.left) as f32) - (WIDTH/2.0)
                    } else {
                        (((event.icon_rect.right - event.icon_rect.left)/2.0 + event.icon_rect.left) as f32) - WIDTH
                    }
                };
                let y = {
                    if cfg!(windows) {
                        (event.icon_rect.top as f32) - 70.0 - HEIGHT
                    } else {
                        (event.icon_rect.top as f32) + 12.0
                    }
                };
                    
                let pos = Pos2::new(x, y);
                frame.set_window_pos(pos);
            }
        }
        
        frame.set_visible(self.is_visible);

        custom_window_frame(ctx, frame, |ui| {
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

fn custom_window_frame(
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    use egui::*;

    let panel_frame = egui::Frame {
        fill: ctx.style().visuals.window_fill(),
        rounding: 10.0.into(),
        stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
        outer_margin: 0.5.into(), // so the stroke is within the bounds
        ..Default::default()
    };

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        let app_rect = ui.max_rect();

        // Add the contents:
        let content_rect = app_rect.shrink(4.0);
        let mut content_ui = ui.child_ui(content_rect, *ui.layout());
        add_contents(&mut content_ui);
    });
}


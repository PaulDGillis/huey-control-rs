#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crossbeam_channel::Receiver;
use eframe::{egui::{self, CentralPanel}, Storage, epaint::{Pos2, Vec2}, IconData};
use light_rs_core::{HueError, HueBridge};
use light_view::LightsViewModel;
use poll_promise::Promise;
use tray_icon::{TrayIconBuilder, TrayEvent, ClickEvent, TrayIcon, menu::{Menu, MenuItem, MenuEvent}};

mod light_view;
mod toggle_switch;

pub type Result<T> = std::result::Result<T, HueError>;

const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 200.0;
const BRIDGE_IP_KEY: &'static str = "bridge_ip";
const BRIDGE_KEY_KEY: &'static str = "bridge_key";

#[tokio::main]
async fn main() -> std::result::Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    // Load /icon.png into memory for window icon and tray icon
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png");
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    let icon = tray_icon::icon::Icon::from_rgba(icon_rgba.clone(), icon_width, icon_height)
        .expect("Failed to open icon");

    let icon_data = Some(IconData { rgba: icon_rgba, width: icon_width, height: icon_height });

    // Create eframe winit window options and run app.
    let options = eframe::NativeOptions {
        resizable: false,
        decorated: false,
        transparent: true,
        min_window_size: Some(Vec2::new(WIDTH, HEIGHT)),
        max_window_size: Some(Vec2::new(WIDTH, HEIGHT)),
        always_on_top: true,
        run_and_return: false,
        icon_data,
        ..Default::default()
    };

    eframe::run_native(
        "Light-rs",
        options,
        Box::new(move |cc| Box::new(LightRS::new(cc, icon))),
    )
}

struct LightRS {
    is_visible: bool,
    bridge_ip: Option<Promise<Result<String>>>,
    bridge: Option<Promise<Result<HueBridge>>>,
    light_viewmodel: LightsViewModel,
    tray_receiver: Receiver<TrayEvent>,
    menu_receiver: Receiver<MenuEvent>,
    _tray_icon: TrayIcon
}

impl LightRS {
    fn new(_cc: &eframe::CreationContext<'_>, icon: tray_icon::icon::Icon) -> LightRS {
        // Load previous bridge from storage if possible.
        let bridge = _cc.storage.and_then(|store| {
            if let Some(bridge_ip) = store.get_string(BRIDGE_IP_KEY) {
                if let Some(bridge_key) = store.get_string(BRIDGE_KEY_KEY) {
                    return Some(HueBridge::new(bridge_ip, bridge_key));
                }
            }
            None
        });

        // Convert storage values to promises
        let (bridge_ip, bridge) = if let Some(bridge) = bridge {
            (Some(Promise::from_ready(Ok(bridge.bridge_ip.clone()))),
            Some(Promise::from_ready(Ok(bridge))))
        } else { (None, None) };

        // Create quit tray button for windows/linux
        let quit = MenuItem::new("Quit", true, None);

        // Create tray icon
        let tray_icon = {
            let mut builder = TrayIconBuilder::new()
                .with_tooltip("Light-rs")
                .with_icon(icon);

            // Create tray menu (For quit button on windows/linux, not added on macOS)
            if !cfg!(target_os = "macos") {
                let menu = Menu::with_items(&[&quit]);
                builder = builder.with_menu(Box::new(menu));
            }

            builder.build().unwrap()
        };

        // Wrap tray icon event handler to trigger update on event
        let context = _cc.egui_ctx.clone();
        let (s, tray_receiver) = crossbeam_channel::unbounded();
        #[allow(unused_must_use)]
        TrayEvent::set_event_handler(Some(move |item| {
            s.send(item);
            context.request_repaint();
        }));

        // Wrap tray menu button event handler to trigger update on event
        let quit_id = quit.id();
        let context = _cc.egui_ctx.clone();
        let (s, menu_receiver) = crossbeam_channel::unbounded();
        #[allow(unused_must_use)]
        MenuEvent::set_event_handler(Some(move |item: MenuEvent| {
            if item.id == quit_id {
                s.send(item);
                context.request_repaint();
            }
        }));

        // Initial app
        LightRS {
            is_visible: false,
            bridge_ip,
            bridge,
            light_viewmodel: LightsViewModel::new(),
            tray_receiver,
            menu_receiver,
            _tray_icon: tray_icon
        }
    }

    fn pair_bridge(bridge_ip: &String) -> Promise<Result<HueBridge>> {
        let bridge_ip = bridge_ip.clone();
        Promise::spawn_async(async move { 
            HueBridge::pair(bridge_ip).await
        })
    }
}

impl eframe::App for LightRS {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process menu events
        if self.menu_receiver.try_recv().is_ok() { frame.close(); }

        // Process tray events
        if let Ok(event) = self.tray_receiver.try_recv() {
            if event.event == ClickEvent::Left {
                self.is_visible = !self.is_visible;

                let ppi = ctx.pixels_per_point();
                let (x, y) = {
                    let icon_center_x = ((event.icon_rect.right - event.icon_rect.left) / 2.0 + event.icon_rect.left) as f32;
                    let icon_top_y = event.icon_rect.top as f32;
                    if cfg!(windows) {
                        (icon_center_x - (WIDTH / 2.0), icon_top_y - (70.0 * ppi) - HEIGHT)
                    } else {
                        (icon_center_x - WIDTH, icon_top_y + 12.0)
                    }
                };
                    
                let pos = Pos2::new(x, y);
                frame.set_window_pos(pos);
            }
        }
        
        frame.set_visible(self.is_visible);

        // Draw ui
        // Create rounded window frame
        let panel_frame = egui::Frame {
            fill: ctx.style().visuals.window_fill(),
            rounding: 10.0.into(),
            stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
            outer_margin: 0.5.into(), // so the stroke is within the bounds
            ..Default::default()
        };
    
        CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            // Shrink ui to fit in rounded panel_frame
            let app_rect = ui.max_rect();
            let content_rect = app_rect.shrink(4.0);
            let ui = &mut ui.child_ui(content_rect, *ui.layout());

            // Either get the already discovered bridge_ip or create an async request to look for one.
            let bridge_ip = self.bridge_ip.get_or_insert_with(|| {
                Promise::spawn_async(async move { HueBridge::discover().await })
            });

            // Display spinner until we get bridge_ip. TODO: Error should have a retry.
            let Some(Ok(bridge_ip)) = bridge_ip.ready() else {
                ui.spinner();
                return;
            };

            // Either get the pair key or create an async request for one.
            let bridge = self.bridge.get_or_insert_with(|| { Self::pair_bridge(&bridge_ip) });
    
            // Show ui spinner until we get a result from async promise.
            let Some(bridge_result) = bridge.ready_mut() else {
                ui.spinner();
                return;
            };

            if let Ok(bridge) = bridge_result {
                // Display light_view.rs when bridge is connected and paired.
                self.light_viewmodel.ui(bridge, ui);
            } else {
                // Display retry ui on error.
                ui.vertical(|ui| {
                    ui.spinner();
                    if ui.button("Retry?").clicked() {
                        self.bridge = Some(Self::pair_bridge(&bridge_ip));
                    }
                });
            }
        });
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        // If bridge is connected, save known ip and api key to storage.
        let Some(bridge_key_opt) = &self.bridge else { return; };
        let Some(Ok(bridge)) = bridge_key_opt.ready() else { return; };
        _storage.set_string(BRIDGE_IP_KEY, bridge.bridge_ip.clone());
        _storage.set_string(BRIDGE_KEY_KEY, bridge.username.clone());
    }
}

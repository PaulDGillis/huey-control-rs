use iced::window::Icon;
use iced_aw::Spinner;
use iced::widget::{column,container};
use iced::{ Application, executor, Length, Theme, Command, Settings, Element };
use huey_core::{HueBridge, HueError, light::Light};
use iced_native::{command::Action, window};
use tray::{HueyTray, HueyTrayEvent};
use tray_icon::ClickEvent;

mod tray;

const WIDTH: u32 = 300;
const HEIGHT: u32 = 200;

fn main() -> iced::Result {
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

    let window_icon = Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon2");

    let settings = Settings {
        window: iced::window::Settings {
            icon: Some(window_icon),
            min_size: Some((WIDTH, HEIGHT)),
            max_size: Some((WIDTH, HEIGHT)),
            always_on_top: true,
            decorations: false,
            transparent: true,
            ..iced::window::Settings::default()
        },
        ..Settings::with_flags(icon)
    };

    HueyApp::run(settings)
}

enum State {
    Search,
    Paired,
    LinkButtonNotPressed,
    Failed,
    #[allow(dead_code)]
    Dashboard { lights: Vec<Light> }
}

#[derive(Debug)]
pub enum Message {
    DiscoverBridge(Result<String, HueError>),
    PairBridge(Result<HueBridge, HueError>),
    ListLights(Result<Vec<Light>, HueError>),
    TrayEvent(HueyTrayEvent),
    ChangePower(bool),
    ChangeColor,
    None
}

struct HueyApp {
    tray: HueyTray,
    bridge: Option<HueBridge>,
    state: State
}

impl Application for HueyApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = tray_icon::icon::Icon;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        (Self {
            tray: HueyTray::new(flags),
            bridge: None,
            state: State::Search,
        }, Command::perform(HueBridge::discover(), Message::DiscoverBridge))
    }

    fn title(&self) -> String {
        "HueyApp".into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        self.tray.tray_worker()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::DiscoverBridge(result) => {
                if let Ok(ip) = result {
                    // return Command::perform(HueBridge::pair(ip), Message::PairBridge);
                } else { 
                    self.state = State::Failed;
                }
            },
            Message::PairBridge(result) => {
                match result {
                    Ok(bridge) => {
                        self.bridge = Some(bridge.clone());
                        self.state = State::Paired;
                        // return Command::perform(Light::list_lights(bridge), Message::ListLights);
                    },
                    Err(HueError::LinkButtonNotPressed) => {
                        self.state = State::LinkButtonNotPressed;
                    },
                    _ => { self.state = State::Failed }
                }
            },
            Message::ListLights(result) => {
                self.state = if let Ok(lights) = result {
                    State::Dashboard { lights }
                } else {
                    State::Failed
                }
            },
            Message::TrayEvent(HueyTrayEvent::TrayEvent(event)) => {
                if event.event == ClickEvent::Left {
                    let icon_rect = event.icon_rect;
                    
                }
                println!("{:?}", event);
            },
            Message::TrayEvent(HueyTrayEvent::MenuEvent(_)) => {
                return Command::single(Action::Window(window::Action::Close))
            },
            _ => {},
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        match self {
            // LightRS::BridgeSearch => todo!(),
            // LightRS::BridgeFound => todo!(),
            // LightRS::BridgePaired { bridge } => todo!(),
            // LightRS::LinkButtonNotPressed => todo!(),
            // LightRS::Failed => todo!(),
            // LightRS::Dashboard { bridge, lights } => todo!(),
            _ => {
                column![
                    container(Spinner::new())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y(),
                ].into()
            }
        }
        
        // column![
        //     text("test").size(50),
        //     // toggler("test".to_string(), false, |_| { Message::Toggle })
        // ]
        // .padding(10)
        // .align_items(Alignment::Center)
        // .into()
    }
}

use iced_aw::Spinner;
use iced::widget::{column,container};
use iced::{ Application, executor, Length, Theme, Command, Settings, Element };
use huey_core::{HueBridge, HueError, light::Light};
use tray::HueyTray;
use tray_icon::TrayEvent;

mod tray;

fn main() -> iced::Result {
    HueyApp::run(Settings::default())
}

enum HueyState {
    BridgeSearch,
    BridgeFound,
    BridgePaired { bridge: HueBridge },
    LinkButtonNotPressed,
    Failed,
    #[allow(dead_code)]
    Dashboard { bridge: HueBridge, lights: Vec<Light> }
}

struct HueyApp {
    tray: HueyTray,
    state: HueyState
}

#[derive(Debug)]
pub enum Message {
    DiscoverBridge(Result<String, HueError>),
    PairBridge(Result<HueBridge, HueError>),
    ListLights(Result<Vec<Light>, HueError>),
    TrayEvent(TrayEvent),
    ChangePower(bool),
    ChangeColor,
    None
}

impl Application for HueyApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (Self {
            tray: HueyTray::new(),
            state: HueyState::BridgeSearch
        }, Command::perform(HueBridge::discover(), Message::DiscoverBridge))
    }

    fn title(&self) -> String {
        "Light-rs".into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        self.tray.tray_worker()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::DiscoverBridge(result) => {
                if let Ok(ip) = result {
                    self.state = HueyState::BridgeFound;
                    return Command::perform(HueBridge::pair(ip), Message::PairBridge);
                } else { 
                    self.state = HueyState::Failed;
                }
            },
            Message::PairBridge(result) => {
                match result {
                    Ok(bridge) => {
                        self.state = HueyState::BridgePaired { bridge: bridge.clone() };
                        // return Command::perform(Light::list_lights(&bridge), Message::ListLights)
                    },
                    Err(HueError::LinkButtonNotPressed) => {
                        self.state = HueyState::LinkButtonNotPressed;
                    },
                    _ => { self.state = HueyState::Failed }
                }
            },
            Message::ListLights(result) => {
                if let Ok(lights) = result {
                    self.state = if let HueyState::BridgePaired { bridge} = &self.state {
                        HueyState::Dashboard { bridge: bridge.to_owned(), lights }
                    } else {
                        HueyState::Failed
                    };
                } else {
                    self.state = HueyState::Failed;
                }
            },
            Message::TrayEvent(event) => {
                println!("{:?}", event);
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

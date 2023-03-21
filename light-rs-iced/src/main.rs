use iced_aw::Spinner;
use iced::widget::{column,text,toggler,container};
use iced::{ Application, executor, Length, Theme, Command, Settings, Element, Alignment };
use light_rs_core::light::Light;
use light_rs_core::{HueBridge, HueError};

fn main() -> iced::Result {
    LightRS::run(Settings::default())
}

enum LightRS {
    BridgeSearch,
    BridgeFound,
    BridgePaired { bridge: HueBridge },
    LinkButtonNotPressed,
    Failed,
    Dashboard { bridge: HueBridge, lights: Vec<Light> }
}

#[derive(Debug)]
enum Message {
    DiscoverBridge(Result<String, HueError>),
    PairBridge(Result<HueBridge, HueError>),
    ListLights(Result<Vec<Light>, HueError>),
    ChangePower(bool),
    ChangeColor
}

impl Application for LightRS {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (Self::BridgeSearch, Command::perform(HueBridge::discover(), Message::DiscoverBridge))
    }

    fn title(&self) -> String {
        "Light-rs".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::DiscoverBridge(result) => {
                if let Ok(ip) = result {
                    *self = Self::BridgeFound;
                    return Command::perform(HueBridge::pair(ip), Message::PairBridge);
                } else { 
                    *self = Self::Failed;
                }
            },
            Message::PairBridge(result) => {
                match result {
                    Ok(bridge) => {
                        *self = Self::BridgePaired { bridge: bridge.clone() };
                        return Command::perform(Light::list_lights(bridge), Message::ListLights)
                    },
                    Err(HueError::LinkButtonNotPressed) => {
                        *self = Self::LinkButtonNotPressed;
                    },
                    _ => { *self = Self::Failed }
                }
            },
            Message::ListLights(result) => {
                if let Ok(lights) = result {
                    if let Self::BridgePaired { bridge} = self {
                        *self = Self::Dashboard { bridge: bridge.to_owned(), lights }
                    } else {
                        *self = Self::Failed;
                    }
                } else {
                    *self = Self::Failed;
                }
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

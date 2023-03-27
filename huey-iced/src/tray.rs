use crossbeam_channel::Receiver;
use iced::{Subscription, subscription};
// use iced::{Subscription, subscription};
use tray_icon::{TrayEvent, TrayIcon};
use crate::Message;

pub struct HueyTray {
    _tray_icon: TrayIcon,
    receiver: Receiver<TrayEvent>
}

impl HueyTray {
    pub fn new() -> Self {
        let _tray_icon = tray_icon::TrayIconBuilder::new()
            .with_tooltip("system-tray - tray icon library!")
            .build()
            .unwrap();
    
        let (sender, receiver) = crossbeam_channel::unbounded();
        #[allow(unused_must_use)]
        TrayEvent::set_event_handler(Some(move |event| {
            sender.send(event);
        }));

        Self { _tray_icon, receiver }
    }

    pub fn tray_worker(&self) -> Subscription<Message> {
        struct TrayWorker;

        subscription::unfold(
            std::any::TypeId::of::<TrayWorker>(),
            self.receiver.clone(),
            |receiver| async move {
                if let Ok(event) = receiver.recv() {
                    (Some(Message::TrayEvent(event)), receiver)
                } else {
                    (None, receiver)
                }
            },
        )
    }
}
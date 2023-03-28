use crossbeam_channel::Receiver;
use iced::{Subscription, subscription};
use tray_icon::{TrayEvent, TrayIcon, menu::{MenuItem, Menu, MenuEvent}, TrayIconBuilder};
use crate::Message;

#[derive(Debug)]
pub enum HueyTrayEvent {
    MenuEvent(MenuEvent),
    TrayEvent(TrayEvent)
}

pub struct HueyTray {
    _tray_icon: TrayIcon,
    tray_recv: Receiver<TrayEvent>,
    menu_recv: Receiver<MenuEvent>,
}

impl HueyTray {
    pub fn new(icon: tray_icon::icon::Icon) -> Self {
        let _tray_icon = if cfg!(target_os = "macos") {
            TrayIconBuilder::new()
                .with_tooltip("Huey")
                .with_icon(icon)
                .build().unwrap()
        } else {
            // Create tray menu (For quit button on windows/linux, not added on macOS)
            let quit = MenuItem::new("Quit", true, None);
            let menu = Menu::with_items(&[&quit]);

            TrayIconBuilder::new()
                .with_tooltip("Huey")
                .with_icon(icon)
                .with_menu(Box::new(menu))
                .build().unwrap()
        };

        Self { 
            _tray_icon, 
            tray_recv: TrayEvent::receiver().clone(), 
            menu_recv: MenuEvent::receiver().clone() 
        }
    }

    pub fn tray_worker(&self) -> Subscription<Message> {
        struct TrayWorker;

        // Dumb to spawn a thread just to listen to events on the main thread, but this only way it will let me.
        subscription::unfold(
            std::any::TypeId::of::<TrayWorker>(),
            (self.tray_recv.clone(), self.menu_recv.clone()),
            |(tray_recv, menu_recv)| async move {
                if let Ok(event) = menu_recv.try_recv() {
                    return (Some(
                        Message::TrayEvent(HueyTrayEvent::MenuEvent(event)
                    )), (tray_recv, menu_recv));
                }
                
                if let Ok(event) = tray_recv.try_recv() {
                    return (Some(
                        Message::TrayEvent(HueyTrayEvent::TrayEvent(event)
                    )), (tray_recv, menu_recv));
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                (None, (tray_recv, menu_recv))
            },
        )
    }
}
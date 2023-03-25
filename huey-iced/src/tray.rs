use crossbeam_channel::Receiver;
// use iced::{Subscription, subscription};
use tray_icon::TrayEvent;

// use crate::HueyApp;

pub enum ExtTrayEvent {
    Ready,
    Done(TrayEvent),
}
enum State {
    Starting,
    Ready(Receiver<TrayEvent>),
}

// pub fn tray_worker(app: &HueyApp) -> Subscription<TrayEvent> {
//     struct TrayWorker;

//     let _tray_icon = tray_icon::TrayIconBuilder::new()
//         .with_tooltip("system-tray - tray icon library!")
//         .build()
//         .unwrap();

//     let (sender, receiver) = crossbeam_channel::unbounded();
//     TrayEvent::set_event_handler(Some(move |event| {
//         sender.send(event);
//     }));

//     return Subscription::with(app, TrayEvent::receiver().try_recv().unwrap()).0;

    // subscription::unfold(std::any::TypeId::of::<TrayWorker>(), State::Starting, |state| async move {
    //     match state {
    //         State::Starting => {
    //             // Create channel
                
    //             (Some(ExtTrayEvent::Ready), State::Ready(receiver))
    //         }
    //         State::Ready(receiver) => {
    //             // Read next input sent from `Application`
    //             match receiver.try_recv() {
    //                 Ok(tray_event) => {
    //                     // Do some async work...

    //                     // Finally, we can optionally return a message to tell the
    //                     // `Application` the work is done
    //                     (Some(ExtTrayEvent::Done(tray_event)), State::Ready(receiver))
    //                 }
    //                 _ => { (Some(ExtTrayEvent::Ready), State::Ready(receiver)) }
    //             }
    //         }
    //     }
    // })
// }
use huebridge::HueBridge;

mod huebridge;

fn main() {
    let discover_res = HueBridge::discover();

    if let Ok(bridge) = discover_res {
        let client = reqwest::blocking::Client::builder().danger_accept_invalid_certs(true).build().expect("Test");

        bridge.pair(client);
    }
}

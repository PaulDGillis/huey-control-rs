use huebridge::HueBridge;

mod huebridge;

fn main() {
    let bridge_addr = HueBridge::discover().unwrap();

    let client = reqwest::blocking::Client::builder().danger_accept_invalid_certs(true).build().expect("Test");

    let res = HueBridge::pair(client, bridge_addr);
    println!("Pair = {}", res.is_ok());

    if let Ok(bridge) = res {
        bridge.list_lights();
    }
}

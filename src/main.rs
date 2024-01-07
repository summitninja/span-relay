use reqwest;
use std::env;
use std::thread;
use std::time::Duration;

#[derive(serde::Deserialize)]
struct CircuitData {
    id: String,
    name: String,
    #[serde(rename = "relayState")]
    relay_state: String,
    #[serde(rename = "instantPowerW")]
    instant_power_w: f64,
    #[serde(rename = "instantPowerUpdateTimeS")]
    instant_power_update_time_s: i64,
    #[serde(rename = "producedEnergyWh")]
    produced_energy_wh: f64,
    #[serde(rename = "consumedEnergyWh")]
    consumed_energy_wh: f64,
    #[serde(rename = "energyAccumUpdateTimeS")]
    energy_accum_update_time_s: i64,
    tabs: Vec<i32>,
    priority: String,
    #[serde(rename = "isUserControllable")]
    is_user_controllable: bool,
    #[serde(rename = "isSheddable")]
    is_sheddable: bool,
    #[serde(rename = "isNeverBackup")]
    is_never_backup: bool,
}

fn main() {
    let api_key = env::var("API_KEY").expect("Missing environment variable 'API_KEY' Exiting");
    let span_host =
        env::var("SPAN_HOST").expect("Missing environment variable 'SPAN_HOST' Exiting");
    let influx_username = env::var("INFLUX_USERNAME")
        .expect("Missing environment variable 'INFLUX_USERNAME' Exiting");
    let influx_api_key =
        env::var("INFLUX_API_KEY").expect("Missing environment variable 'INFLUX_API_KEY' Exiting");
    let influx_host =
        env::var("INFLUX_HOST").expect("Missing environment variable 'INFLUX_HOST' Exiting");

    loop {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(format!("{}/api/v1/circuits", &span_host))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .expect("Failed to send request");

        let circuits = response.json::<serde_json::Value>().unwrap();
        let circuits: Vec<CircuitData> = circuits["circuits"]
            .to_owned()
            .as_object()
            .unwrap()
            .to_owned()
            .values()
            .collect::<Vec<&serde_json::Value>>()
            .iter()
            .map(|v| serde_json::from_value(v.to_owned().to_owned()).unwrap())
            .collect();

        let mut influx_payload: Vec<String> = circuits
        .iter()
        .map(|v| {
            format!(
                "{0},source=span instant_power={1}\n{0},source=span consumed_kw={2}\n{0},source=span produced_kw={3}\n{0},source=span state={4}",
                v.name
                    .to_lowercase()
                    .replace(" ", "_")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>(),
                v.instant_power_w,
                v.consumed_energy_wh,
                v.produced_energy_wh,
                if v.relay_state == "CLOSED" { "1" } else { "0" },
            )
        })
        .collect();

        let response = client
            .get(format!("{}/api/v1/panel", &span_host))
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .expect("Failed to send request");

        let panel = response.json::<serde_json::Value>().unwrap();

        influx_payload.push(format!(
            "feed_through,source=span produced={}\nfeed_through,source=span consumed={}",
            panel["feedthroughEnergy"]["producedEnergyWh"],
            panel["feedthroughEnergy"]["consumedEnergyWh"]
        ));

        // upload to influx cloud
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&influx_host)
            .header("Content-Type", "text/plain")
            .basic_auth(&influx_username, Some(&influx_api_key))
            .body(influx_payload.join("\n"))
            .send()
            .expect("Failed to send request");

        println!("Response: {:?}", response.status());

        thread::sleep(Duration::from_secs(60));
    }
}

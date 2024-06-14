use reqwest::Client;

use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str, to_string};
use std::env;
use tokio::spawn;
use tokio::time::interval;
use tokio::time::Duration;
use chrono::DateTime;

const BASE_REQUEST_STRING: &str = "http://lapi.transitchicago.com/api/1.0/ttarrivals.aspx";

#[derive(Debug, Deserialize)]
struct EtaInfo {
    #[serde(rename = "staId")]
    station_id: String,
    #[serde(rename = "arrT")]
    arrival_time: String,
    #[serde(rename = "stpDe")]
    stop_description: String,
}

#[derive(Debug, Deserialize)]
struct TrainInfo {
    #[serde(rename = "eta")]
    eta: Vec<EtaInfo>,
}

fn build_request_string(api_key: &str, stop_id: usize) -> String {
    format!("{BASE_REQUEST_STRING}?stpid={stop_id}&key={api_key}")
}

async fn run_task() {
    let api_key = env::var("TRAIN_API_KEY").expect("need api key");
    let response = reqwest::get(build_request_string(&api_key, 30173)).await.unwrap();

    if response.status().is_success() {
        let body = response.text().await.unwrap();
        let train_info: TrainInfo = serde_xml_rs::from_str(&body).unwrap();
        for eta in train_info.eta {
            println!("{}: {}", eta.stop_description, eta.arrival_time)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = interval(Duration::from_secs(10));

    loop {
        interval.tick().await;
        spawn(async { run_task().await });
    }
}

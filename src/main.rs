use chrono::{NaiveDateTime, Timelike};
use serde::Deserialize;
use std::{env, fs};
use tokio::spawn;
use tokio::time::interval;
use tokio::time::Duration;

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

#[derive(Debug, Deserialize, Clone)]
struct Stop {
    name: String,
    id: usize,
}

#[derive(Debug, Deserialize)]
struct Config {
    stops: Vec<Stop>,
    interval: u64,
}

fn build_request_string(api_key: &str, stop_id: usize) -> String {
    format!("{BASE_REQUEST_STRING}?stpid={stop_id}&key={api_key}")
}

fn format_time(time_str: &str) -> Result<String, String> {
    let date_time = NaiveDateTime::parse_from_str(time_str, "%Y%m%d %H:%M:%S")
        .map_err(|err| format!("failed to parse time: {err}"))?;
    Ok(format!("{}:{}", date_time.hour(), date_time.minute()))
}

fn generate_display_text(train_info: &TrainInfo, stop: &Stop) -> String {
    let train_info_stop = &train_info
        .eta
        .first()
        .map(|info| info.stop_description.clone());
    
    let stop_name = &stop.name;

    train_info_stop.as_ref()
        .map(|s| {
            let times = train_info
                .eta
                .iter()
                .map(|e| format_time(&e.arrival_time).unwrap())
                .collect::<Vec<_>>()
                .join(" ");
            format!("{stop_name}\n{s}\n--\n{times}\n")
        })
        .unwrap_or(format!("{stop_name}\n\n--\nNo trains scheduled"))
}

async fn try_cycle_display(stop: Stop) -> Result<(), String> {
    let api_key = env::var("TRAIN_API_KEY").expect("need api key");
    let response = reqwest::get(build_request_string(&api_key, stop.id))
        .await
        .map_err(|err| format!("failed to make get request: {err}"))?;

    if response.status().is_success() {
        let body = response
            .text()
            .await
            .map_err(|err| format!("failed to get response text: {err}"))?;
        let train_info: TrainInfo = serde_xml_rs::from_str(&body)
            .map_err(|err| format!("failed to serialize response: {err}"))?;

        let display = generate_display_text(&train_info, &stop);
        println!("{display}");

        Ok(())
    } else {
        Err(format!(
            "error code from trains API: {}",
            response.status().as_str()
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let stop_yaml = "stops.yaml";
    let config: Config = serde_yaml::from_str(
        &fs::read_to_string(stop_yaml).map_err(|err| format!("failed to load yaml: {err}"))?,
    )
    .map_err(|err| format!("failed to parse yaml: {err}"))?;

    let mut interval = interval(Duration::from_secs(config.interval));

    let mut current_stop_index = 0;
    loop {
        interval.tick().await;

        let stop = config
            .stops
            .get(current_stop_index)
            .ok_or(format!("invalid config index: {current_stop_index}"))?.clone();
        spawn(async { try_cycle_display(stop).await });

        current_stop_index = (current_stop_index + 1) % config.stops.len();
    }
}

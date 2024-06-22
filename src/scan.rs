use std::error::Error;
use std::str::FromStr;

use btleplug::api::CentralEvent;
use btleplug::api::{Central, Manager as _, ScanFilter};
use btleplug::platform::Manager;
use log::info;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{BATTERY, HUMIDITY, TEMPERATURE};

#[derive(Debug)]
pub struct SensorData {
    pub battery: f64,
    pub temperature: f64,
    pub humidity: f64,
}

pub async fn scan_loop() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters
        .into_iter()
        .next()
        .ok_or("No Bluetooth adapter found")?;

    let mut events = central.events().await?;
    central.start_scan(ScanFilter::default()).await?;
    info!("Start scanning for SwitchBot TH");

    let service_uuid = Uuid::from_str("0000fd3d-0000-1000-8000-00805f9b34fb")?;
    while let Some(event) = events.next().await {
        if let CentralEvent::ServiceDataAdvertisement { service_data, .. } = event {
            if let Some(service_data) = service_data.get(&service_uuid) {
                if service_data.len() != 6 {
                    continue;
                }
                let metrics = parse_service_data(service_data);

                if let Some(battery) = BATTERY.get() {
                    battery.set(metrics.battery);
                }
                if let Some(temperature) = TEMPERATURE.get() {
                    temperature.set(metrics.temperature);
                }
                if let Some(humidity) = HUMIDITY.get() {
                    humidity.set(metrics.humidity);
                }
            }
        }
    }
    central.stop_scan().await?;

    Ok(())
}

fn parse_service_data(data: &[u8]) -> SensorData {
    let battery = (data[2] & 0b0111_1111) as f64;
    let is_temperature_above_freezing = data[4] & 0b1000_0000 > 0;
    let temperature = (data[3] & 0b0000_1111) as f64 * 0.1 + (data[4] & 0b0111_1111) as f64;
    let temperature = if is_temperature_above_freezing {
        temperature
    } else {
        -temperature
    };
    let humidity = (data[5] & 0b0111_1111) as f64;

    SensorData {
        battery,
        temperature,
        humidity,
    }
}

use std::error::Error;
use std::str::FromStr;

use btleplug::api::CentralEvent;
use btleplug::api::{Central, Manager as _, ScanFilter};
use btleplug::platform::Manager;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        let manager = Manager::new().await.unwrap();

        // get the first bluetooth adapter
        let adapters = manager.adapters().await?;
        let central = adapters.into_iter().nth(0).unwrap();

        // start scanning for devices
        let mut events = central.events().await?;
        central.start_scan(ScanFilter::default()).await?;

        while let Some(event) = events.next().await {
            if let CentralEvent::ServiceDataAdvertisement { service_data, .. } = event {
                let now = chrono::Local::now();
                println!("{} Service data: {:?}", now.to_string(), service_data);
                let service_uuid = Uuid::from_str("0000fd3d-0000-1000-8000-00805f9b34fb")?;
                if let Some(service_data) = service_data.get(&service_uuid) {
                    if service_data.len() != 6 {
                        continue;
                    }
                    let metrics = parse_service_data(&service_data);
                    let now = chrono::Local::now();
                    println!(
                        "{} Battery: {}%, Temperature: {}°C, Humidity: {}%",
                        now.to_string(),
                        metrics.battery,
                        metrics.temperature,
                        metrics.humidity
                    );
                }
            }
        }
        central.stop_scan().await?;

        Ok(())
    })
}

#[derive(Debug)]
struct Metrics {
    battery: u8,
    temperature: f64,
    humidity: u8,
}

fn parse_service_data(data: &[u8]) -> Metrics {
    let battery = data[2] & 0b0111_1111;
    let is_temperature_above_freezing = data[4] & 0b1000_0000 > 0;
    let temperature = (data[3] & 0b0000_1111) as f64 * 0.1 + (data[4] & 0b0111_1111) as f64;
    let temperature = if is_temperature_above_freezing {
        temperature
    } else {
        -temperature
    };
    let humidity = data[5] & 0b0111_1111;

    Metrics {
        battery,
        temperature,
        humidity,
    }
}

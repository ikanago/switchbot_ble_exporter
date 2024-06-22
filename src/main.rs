use std::error::Error;
use std::sync::OnceLock;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use prometheus::{register_gauge, Encoder, Gauge};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

mod scan;

static BATTERY: OnceLock<Gauge> = OnceLock::new();
static TEMPERATURE: OnceLock<Gauge> = OnceLock::new();
static HUMIDITY: OnceLock<Gauge> = OnceLock::new();

fn main() -> Result<(), Box<dyn Error>> {
    BATTERY.get_or_init(|| {
        register_gauge!("battery", "Battery level of SwitchBot TH in percent").unwrap()
    });
    TEMPERATURE.get_or_init(|| {
        register_gauge!(
            "temperature",
            "Temperature measured by SwitchBot TH in celcius"
        )
        .unwrap()
    });
    HUMIDITY.get_or_init(|| {
        register_gauge!(
            "humidity",
            "Relative humudity measured by SwitchBot TH in percent"
        )
        .unwrap()
    });

    let rt = Runtime::new()?;
    rt.block_on(async {
        tokio::spawn(async move {
            scan::scan_loop().await.unwrap();
        });

        let app = Router::new()
            .route("/metrics", get(handler))
            .fallback(|| async { StatusCode::NOT_FOUND });
        let listener = TcpListener::bind(("0.0.0.0", 8080)).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    Ok(())
}

async fn handler() -> (StatusCode, String) {
    let metrics_families = prometheus::gather();
    let mut buffer = vec![];
    let encoder = prometheus::TextEncoder::new();
    if let Err(err) = encoder.encode(&metrics_families, &mut buffer) {
        eprintln!("Failed to encode metrics: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Internal server error\n"),
        );
    }

    let buffer = String::from_utf8(buffer).unwrap();
    (StatusCode::OK, buffer)
}

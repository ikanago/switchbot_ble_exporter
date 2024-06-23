use std::error::Error;
use std::sync::OnceLock;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use log::{error, info};
use prometheus::{register_gauge, Encoder, Gauge};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

mod scan;

static BATTERY: OnceLock<Gauge> = OnceLock::new();
static TEMPERATURE: OnceLock<Gauge> = OnceLock::new();
static HUMIDITY: OnceLock<Gauge> = OnceLock::new();
static VPD: OnceLock<Gauge> = OnceLock::new();
static DISCOMFORT_INDEX: OnceLock<Gauge> = OnceLock::new();

fn main() -> Result<(), Box<dyn Error>> {
    BATTERY.get_or_init(|| {
        register_gauge!(
            "switchbot_th_battery",
            "Battery level of SwitchBot TH in percent"
        )
        .unwrap()
    });
    TEMPERATURE.get_or_init(|| {
        register_gauge!(
            "switchbot_th_temperature",
            "Temperature measured by SwitchBot TH in celcius"
        )
        .unwrap()
    });
    HUMIDITY.get_or_init(|| {
        register_gauge!(
            "switchbot_th_humidity",
            "Relative humudity measured by SwitchBot TH in percent"
        )
        .unwrap()
    });
    VPD.get_or_init(|| {
        register_gauge!(
            "switchbot_th_vpd",
            "Vapor pressure deficit calculated from temperature and relative humidity"
        )
        .unwrap()
    });
    DISCOMFORT_INDEX.get_or_init(|| {
        register_gauge!(
            "switchbot_th_discomfort_index",
            "Discomfort index calculated from temperature and relative humidity"
        )
        .unwrap()
    });

    let rt = Runtime::new()?;
    rt.block_on(async {
        env_logger::init();

        tokio::spawn(async move {
            if let Err(err) = scan::scan_loop().await {
                error!("Failed to scan: {}", err);
            }
        });

        let app = Router::new()
            .route("/metrics", get(handler))
            .fallback(landing);
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap();
        let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
        info!("Listening on: {}", listener.local_addr().unwrap());
        axum::serve(listener, app).await.unwrap();
    });
    Ok(())
}

async fn handler() -> (StatusCode, String) {
    let metrics_families = prometheus::gather();
    let mut buffer = vec![];
    let encoder = prometheus::TextEncoder::new();
    if let Err(err) = encoder.encode(&metrics_families, &mut buffer) {
        error!("Failed to encode metrics: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Internal server error\n"),
        );
    }

    let buffer = String::from_utf8(buffer).unwrap();
    (StatusCode::OK, buffer)
}

async fn landing() -> impl IntoResponse {
    let header = [("Content-Type", "text/html")];
    let body = r#"<html>
<head><title>SwitchBot TH Exporter</title></head>
<body>
<h1>SwitchBot TH Exporter</h1>
<p><a href="/metrics">Metrics</a></p>
</body>
"#;
    (StatusCode::OK, header, body)
}

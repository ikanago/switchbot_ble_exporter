use std::error::Error;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use scan::SensorData;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

mod scan;

type SharedSensorData = Arc<Mutex<Option<SensorData>>>;

fn main() -> Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        let latest_metrics = Arc::new(Mutex::new(None));
        let latest_metrics_clone = latest_metrics.clone();
        tokio::spawn(async move {
            scan::scan_loop(latest_metrics_clone).await.unwrap();
        });

        let app = Router::new()
            .route("/metrics", get(handler))
            .fallback(|| async { StatusCode::NOT_FOUND })
            .with_state(latest_metrics);
        let listener = TcpListener::bind(("0.0.0.0", 8080)).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    Ok(())
}

async fn handler(State(metrics): State<SharedSensorData>) -> (StatusCode, String) {
    let metrics = metrics.lock().unwrap();
    let Some(metrics) = metrics.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("value not available\n"),
        );
    };

    let body = format!(
        "battery {}\ntemperature {}\nhumidity {}\n",
        metrics.battery, metrics.temperature, metrics.humidity
    );
    (StatusCode::OK, body)
}

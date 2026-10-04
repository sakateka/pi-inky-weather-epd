use crate::clock::{Clock, SystemClock};
use crate::configs::settings::DashboardSettings;
use crate::logger;
use crate::utils::{convert_png_bytes_to_raw_7color, convert_svg_to_png_bytes};
use crate::weather_dashboard::generate_dashboard_svg_string;
use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Timelike;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub async fn run_server(settings: DashboardSettings, port: u16) -> Result<(), anyhow::Error> {
    logger::init(settings.dev.enable_debug_logs, settings.misc.timezone);
    let app = Router::new()
        .route("/dashboard.svg", get(serve_svg))
        .route("/dashboard.png", get(serve_png))
        .route("/dashboard.raw", get(serve_raw))
        .route("/static/*path", get(serve_static))
        .with_state(Arc::new(settings));

    let addr = format!("0.0.0.0:{}", port);
    println!("Starting web server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Calculate the X-Next-Delay header value in seconds based on current time and configuration
fn calculate_next_delay(settings: &DashboardSettings, clock: &dyn Clock) -> u32 {
    let active_start = settings.web_server.active_hours_start;
    let active_end = settings.web_server.active_hours_end;
    let active_interval = settings.web_server.active_hours_interval_seconds;

    let now = clock.now_local(settings.misc.timezone);
    let current_hour = now.hour() as u8;

    // Check if we're in active hours (9:00-21:00)
    if current_hour >= active_start && current_hour < active_end {
        // During active hours: return configured interval (default 1 hour)
        active_interval
    } else {
        // Outside active hours: calculate seconds until next active period starts
        let target_hour = active_start as u32;
        let current_hour = now.hour();
        let current_minute = now.minute();
        let current_second = now.second();

        // Calculate seconds until the start of the next active period
        if current_hour < target_hour {
            // Same day - calculate time until active_start
            let hours_diff = target_hour - current_hour;
            (hours_diff * 3600) - (current_minute * 60) - current_second
        } else {
            // Next day - calculate time until tomorrow's active_start
            let hours_until_midnight = 24 - current_hour;
            let seconds_until_midnight =
                (hours_until_midnight * 3600) - (current_minute * 60) - current_second;
            seconds_until_midnight + (target_hour * 3600)
        }
    }
}

/// Create headers with X-Next-Delay for dashboard responses
fn create_dashboard_headers(settings: &DashboardSettings, content_type: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());

    let next_delay = calculate_next_delay(settings, &SystemClock);
    logger::info(format!(
        "Calculated next delay: {:?}",
        Duration::from_secs(next_delay.into())
    ));
    headers.insert("X-Next-Delay", next_delay.to_string().parse().unwrap());

    headers
}

async fn serve_svg(State(settings): State<Arc<DashboardSettings>>) -> Response {
    serve_dashboard(settings, "image/svg+xml", generate_svg_data).await
}

async fn serve_png(State(settings): State<Arc<DashboardSettings>>) -> Response {
    serve_dashboard(settings, "image/png", generate_png_data).await
}

async fn serve_raw(State(settings): State<Arc<DashboardSettings>>) -> Response {
    serve_dashboard(settings, "application/octet-stream", generate_raw_data).await
}

async fn serve_dashboard(
    settings: Arc<DashboardSettings>,
    content_type: &'static str,
    generate: fn(&DashboardSettings) -> Result<Vec<u8>, anyhow::Error>,
) -> Response {
    let render_settings = Arc::clone(&settings);
    // Blocking HTTP clients and rasterization must run outside Tokio's async workers.
    let result = tokio::task::spawn_blocking(move || generate(&render_settings))
        .await
        .unwrap_or_else(|e| Err(e.into()));
    match result {
        Ok(data) => (create_dashboard_headers(&settings, content_type), data).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate dashboard: {e}"),
        )
            .into_response(),
    }
}

fn generate_svg_data(settings: &DashboardSettings) -> Result<Vec<u8>, anyhow::Error> {
    generate_dashboard_svg_string(settings, &SystemClock).map(String::into_bytes)
}

fn generate_png_data(settings: &DashboardSettings) -> Result<Vec<u8>, anyhow::Error> {
    let svg_data = generate_dashboard_svg_string(settings, &SystemClock)?;
    convert_svg_to_png_bytes(&svg_data, settings.misc.png_scale_factor)
}

fn generate_raw_data(settings: &DashboardSettings) -> Result<Vec<u8>, anyhow::Error> {
    convert_png_bytes_to_raw_7color(&generate_png_data(settings)?)
}

async fn serve_static(Path(path): Path<String>) -> Response {
    if std::path::Path::new(&path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let file_path = PathBuf::from("static").join(&path);

    match tokio::fs::read(&file_path).await {
        Ok(contents) => {
            let content_type = if path.ends_with(".svg") {
                "image/svg+xml"
            } else if path.ends_with(".png") {
                "image/png"
            } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                "image/jpeg"
            } else if path.ends_with(".css") {
                "text/css"
            } else if path.ends_with(".js") {
                "application/javascript"
            } else {
                "application/octet-stream"
            };

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, content_type)],
                contents,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, format!("File not found: {}", path)).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;

    #[test]
    fn refresh_delay_uses_the_configured_timezone_and_clock() {
        let mut settings = DashboardSettings::load_test_config().unwrap();
        settings.misc.timezone = chrono_tz::Europe::Moscow;
        for (timestamp, expected) in [
            ("2025-01-01T05:59:30Z", 30),
            ("2025-01-01T06:00:00Z", 3600),
            ("2025-01-01T17:59:59Z", 3600),
            ("2025-01-01T18:00:00Z", 12 * 3600),
        ] {
            let clock = FixedClock::from_rfc3339(timestamp).unwrap();
            assert_eq!(calculate_next_delay(&settings, &clock), expected);
        }
    }
}

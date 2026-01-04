use crate::clock::SystemClock;
use crate::logger;
use crate::utils::{convert_png_bytes_to_raw_7color, convert_svg_to_png_bytes};
use crate::weather_dashboard::generate_dashboard_svg_string;
use crate::CONFIG;
use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::{Local, Timelike};
use std::path::PathBuf;
use std::time::Duration;

pub async fn run_server(port: u16) -> Result<(), anyhow::Error> {
    let app = Router::new()
        .route("/dashboard.svg", get(serve_svg))
        .route("/dashboard.png", get(serve_png))
        .route("/dashboard.raw", get(serve_raw))
        .route("/static/*path", get(serve_static));

    let addr = format!("0.0.0.0:{}", port);
    println!("Starting web server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Calculate the X-Next-Delay header value in seconds based on current time and configuration
fn calculate_next_delay() -> u32 {
    let active_start = CONFIG.web_server.active_hours_start;
    let active_end = CONFIG.web_server.active_hours_end;
    let active_interval = CONFIG.web_server.active_hours_interval_seconds;

    let now = Local::now();
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
        let seconds_until_target = if current_hour < target_hour {
            // Same day - calculate time until active_start
            let hours_diff = target_hour - current_hour;
            (hours_diff * 3600) - (current_minute * 60) - current_second
        } else {
            // Next day - calculate time until tomorrow's active_start
            let hours_until_midnight = 24 - current_hour;
            let seconds_until_midnight =
                (hours_until_midnight * 3600) - (current_minute * 60) - current_second;
            seconds_until_midnight + (target_hour * 3600)
        };

        seconds_until_target
    }
}

/// Create headers with X-Next-Delay for dashboard responses
fn create_dashboard_headers(content_type: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());

    let next_delay = calculate_next_delay();
    logger::info(format!(
        "Calculated next delay: {:?}",
        Duration::from_secs(next_delay.into())
    ));
    headers.insert("X-Next-Delay", next_delay.to_string().parse().unwrap());

    headers
}

async fn serve_svg() -> Response {
    match generate_svg_data() {
        Ok(svg_data) => (
            StatusCode::OK,
            create_dashboard_headers("image/svg+xml"),
            svg_data,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate SVG: {}", e),
        )
            .into_response(),
    }
}

async fn serve_png() -> Response {
    match generate_png_data() {
        Ok(png_data) => (
            StatusCode::OK,
            create_dashboard_headers("image/png"),
            png_data,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate PNG: {}", e),
        )
            .into_response(),
    }
}

async fn serve_raw() -> Response {
    match generate_raw_data() {
        Ok(raw_data) => (
            StatusCode::OK,
            create_dashboard_headers("application/octet-stream"),
            raw_data,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate RAW: {}", e),
        )
            .into_response(),
    }
}

fn generate_svg_data() -> Result<String, anyhow::Error> {
    let clock = SystemClock;
    let input_template_name = &CONFIG.misc.template_path;
    generate_dashboard_svg_string(&clock, input_template_name)
}

fn generate_png_data() -> Result<Vec<u8>, anyhow::Error> {
    let svg_data = generate_svg_data()?;
    let png_bytes = convert_svg_to_png_bytes(&svg_data, CONFIG.misc.png_scale_factor)?;
    Ok(png_bytes)
}

fn generate_raw_data() -> Result<Vec<u8>, anyhow::Error> {
    let png_data = generate_png_data()?;
    let raw_bytes = convert_png_bytes_to_raw_7color(&png_data)?;
    Ok(raw_bytes)
}

async fn serve_static(Path(path): Path<String>) -> Response {
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

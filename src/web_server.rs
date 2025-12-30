use crate::clock::SystemClock;
use crate::utils::{convert_png_bytes_to_raw_7color, convert_svg_to_png_bytes};
use crate::weather_dashboard::generate_dashboard_svg_string;
use crate::CONFIG;
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::path::PathBuf;

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

async fn serve_svg() -> Response {
    match generate_svg_data() {
        Ok(svg_data) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "image/svg+xml")],
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
            [(header::CONTENT_TYPE, "image/png")],
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
            [(header::CONTENT_TYPE, "application/octet-stream")],
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

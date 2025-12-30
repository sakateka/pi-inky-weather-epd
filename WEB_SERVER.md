# Web Server Mode

This project now includes a web server mode that serves dashboard images in different formats without writing to the filesystem.

## Building

To build the project with web server support:

```bash
cargo build --features web --release
```

## Running

To run the web server:

```bash
cargo run --features web --release -- --port 8080
```

Or if you've built the binary:

```bash
./target/release/pi-inky-weather-epd --port 8080
```

Default port is 8080 if not specified.

## API Endpoints

The web server provides three endpoints:

### 1. SVG Dashboard
```
GET /dashboard.svg
```
Returns the dashboard as an SVG image.

**Response:**
- Content-Type: `image/svg+xml`
- Body: SVG image data

### 2. PNG Dashboard
```
GET /dashboard.png
```
Returns the dashboard as a PNG image (scaled according to config).

**Response:**
- Content-Type: `image/png`
- Body: PNG image data

### 3. RAW Dashboard
```
GET /dashboard.raw
```
Returns the dashboard as raw 4-bit color data for e-ink displays.

**Response:**
- Content-Type: `application/octet-stream`
- Body: Raw 4-bit packed color data (7-color palette)

## Examples

Using curl:

```bash
# Download SVG
curl http://localhost:8080/dashboard.svg -o dashboard.svg

# Download PNG
curl http://localhost:8080/dashboard.png -o dashboard.png

# Download RAW
curl http://localhost:8080/dashboard.raw -o dashboard.raw
```

Using a web browser:
- Open `http://localhost:8080/dashboard.svg` to view the SVG
- Open `http://localhost:8080/dashboard.png` to view the PNG

## Features

- **In-memory processing**: All image generation happens in memory, no filesystem writes
- **On-demand generation**: Images are generated fresh on each request
- **Multiple formats**: Supports SVG, PNG, and RAW formats
- **Configurable**: Uses the same configuration as the file-based mode

## Notes

- The web server mode uses the same weather data fetching and dashboard generation logic as the standard mode
- All configuration settings from `config/` are respected
- The server runs asynchronously using Tokio runtime
- Each request generates a fresh dashboard with current weather data
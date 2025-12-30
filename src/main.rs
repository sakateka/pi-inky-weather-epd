use anyhow::Result;

#[cfg(not(any(feature = "cli", feature = "web")))]
use pi_inky_weather_epd::run_weather_dashboard;

// CLI features only available when 'cli' feature is enabled (for simulation/testing)
#[cfg(feature = "cli")]
mod cli {
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use clap::{Parser, Subcommand};
    use pi_inky_weather_epd::{
        clock::FixedClock, render_svg_to_png, run_weather_dashboard,
        run_weather_dashboard_with_clock,
    };
    use std::path::PathBuf;

    fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, String> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                format!(
                    "Invalid RFC3339 timestamp: {e}. Expected format like '2025-12-26T09:00:00Z'"
                )
            })
    }

    #[derive(Subcommand, Debug)]
    enum Command {
        /// Generate the dashboard as if it's a specific time.
        /// Useful for testing time-dependent rendering over a range of hours.
        Simulate {
            /// Fixed timestamp in RFC3339 format (e.g., "2025-12-26T09:00:00Z")
            #[arg(value_name = "TIMESTAMP", value_parser = parse_rfc3339)]
            timestamp: DateTime<Utc>,
        },
        /// Convert an existing SVG file directly to PNG without fetching or re-rendering.
        RenderSvg {
            /// Path to the SVG file to convert
            #[arg(value_name = "SVG_FILE")]
            svg_file: PathBuf,
        },
    }

    /// Pi Inky Weather Display - Generate weather dashboards for e-paper displays
    #[derive(Parser, Debug)]
    #[command(name = "pi-inky-weather-epd")]
    #[command(version, about, long_about = None)]
    struct Args {
        #[command(subcommand)]
        command: Option<Command>,
    }

    pub fn run() -> Result<()> {
        let args = Args::parse();

        match args.command {
            Some(Command::Simulate { timestamp }) => {
                let fixed_clock = FixedClock::new(timestamp);
                run_weather_dashboard_with_clock(&fixed_clock)?;
            }
            Some(Command::RenderSvg { svg_file }) => {
                render_svg_to_png(&svg_file)?;
            }
            None => {
                run_weather_dashboard()?;
            }
        }

        Ok(())
    }
}

// Web server mode
#[cfg(feature = "web")]
mod web {
    use anyhow::Result;
    use clap::Parser;
    use pi_inky_weather_epd::web_server;

    /// Pi Inky Weather Display - Web Server Mode
    #[derive(Parser, Debug)]
    #[command(name = "pi-inky-weather-epd")]
    #[command(version, about, long_about = None)]
    pub struct Args {
        /// Port to run the web server on
        #[arg(short, long, default_value = "8080")]
        pub port: u16,
    }

    pub async fn run() -> Result<()> {
        let args = Args::parse();
        web_server::run_server(args.port).await?;
        Ok(())
    }
}

#[cfg(feature = "cli")]
fn main() -> Result<()> {
    cli::run()
}

#[cfg(feature = "web")]
#[tokio::main]
async fn main() -> Result<()> {
    web::run().await
}

#[cfg(not(any(feature = "cli", feature = "web")))]
fn main() -> Result<()> {
    run_weather_dashboard()?;
    Ok(())
}

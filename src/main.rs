use anyhow::Result;

#[cfg(not(any(feature = "cli", feature = "web")))]
use pi_inky_weather_epd::run_weather_dashboard;

// CLI features only available when 'cli' feature is enabled (for simulation/testing)
#[cfg(feature = "cli")]
mod cli {
    use anyhow::Result;
    use clap::Parser;
    use pi_inky_weather_epd::{
        clock::FixedClock, run_weather_dashboard, run_weather_dashboard_with_clock,
    };

    /// Pi Inky Weather Display - Generate weather dashboards for e-paper displays
    #[derive(Parser, Debug)]
    #[command(name = "pi-inky-weather-epd")]
    #[command(version, about, long_about = None)]
    pub struct Args {
        /// Simulate mode: Use a fixed timestamp (RFC3339 format, e.g., "2025-12-26T09:00:00Z")
        /// When provided, the dashboard will be generated as if it's this time.
        /// Useful for generating multiple dashboards at different times for testing.
        #[arg(long, value_name = "TIMESTAMP")]
        pub simulate_time: Option<String>,
    }

    pub fn run() -> Result<()> {
        let args = Args::parse();

        if let Some(timestamp) = args.simulate_time {
            let fixed_clock = FixedClock::from_rfc3339(&timestamp).map_err(|e| {
                anyhow::anyhow!(
                    "Invalid timestamp format: {}. Expected RFC3339 format like '2025-12-26T09:00:00Z'",
                    e
                )
            })?;
            run_weather_dashboard_with_clock(&fixed_clock)?;
        } else {
            run_weather_dashboard()?;
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

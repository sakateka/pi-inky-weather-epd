use super::validation::*;
use crate::i18n::Language;
use nutype::nutype;
use serde::Deserialize;
use std::{env, fmt, path::PathBuf};
use strum_macros::Display;
use url::Url;

use config::{Config, ConfigError, Environment, File};
const CONFIG_DIR: &str = "./config";
const DEFAULT_CONFIG_NAME: &str = "default";

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Copy, Display)]
#[serde(rename_all = "snake_case")]
pub enum Providers {
    Bom,
    OpenMeteo,
}

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Copy, Display)]
#[serde(rename_all = "UPPERCASE")]
pub enum TemperatureUnit {
    #[strum(serialize = "C")]
    C,
    #[strum(serialize = "F")]
    F,
}

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Copy, Display, Default)]
#[serde(rename_all = "snake_case")]
pub enum HourFormat {
    /// Follow the clock convention of `render_options.language` (12-hour for
    /// English, 24-hour for the other supported languages).
    #[default]
    Auto,
    #[serde(rename = "12h")]
    #[strum(serialize = "12h")]
    TwelveHour,
    #[serde(rename = "24h")]
    #[strum(serialize = "24h")]
    TwentyFour,
}

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Copy, Display)]
pub enum WindSpeedUnit {
    #[serde(rename = "km/h")]
    #[strum(serialize = "km/h")]
    KmH,
    #[serde(rename = "mph")]
    #[strum(serialize = "mph")]
    Mph,
    #[serde(rename = "knots")]
    #[strum(serialize = "knots")]
    Knots,
}

#[nutype(
    sanitize(trim),
    validate(with = is_valid_colour, error = ValidationError),
    derive(Debug, Deserialize, PartialEq, Clone)
)]
pub struct Colour(String);

impl fmt::Display for Colour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.clone().into_inner())
    }
}

#[nutype(
    sanitize(trim, lowercase),
    validate(len_char_min = 6, len_char_max = 6),
    derive(Debug, Deserialize, PartialEq, Clone, AsRef)
)]
pub struct GeoHash(String);

impl fmt::Display for GeoHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.clone().into_inner())
    }
}

#[nutype(
    sanitize(),
    validate(greater_or_equal = 0),
    derive(Debug, Deserialize, PartialEq, Clone, AsRef, Copy)
)]
pub struct UpdateIntervalDays(i32);

impl fmt::Display for UpdateIntervalDays {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_inner())
    }
}

#[nutype(
    sanitize(),
    validate(with = is_valid_longitude, error = ValidationError),
    derive(Debug, Deserialize, PartialEq, Clone, Copy, AsRef)
)]
pub struct Longitude(f64);

impl fmt::Display for Longitude {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_inner())
    }
}

#[nutype(
    sanitize(),
    validate(with = is_valid_latitude, error = ValidationError),
    derive(Debug, Deserialize, PartialEq, Clone, Copy, AsRef)
)]
pub struct Latitude(f64);

impl fmt::Display for Latitude {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_inner())
    }
}

#[nutype(
    sanitize(trim),
    validate(with = is_valid_date_format, error = ValidationError),
    derive(Debug, Deserialize, PartialEq, Clone, AsRef)
)]
pub struct DateFormat(String);

impl fmt::Display for DateFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.clone().into_inner())
    }
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub release_info_url: Url,
    pub download_base_url: Url,
    pub update_interval_days: UpdateIntervalDays,
    pub allow_pre_release_version: bool,
}

#[derive(Debug, Deserialize)]
pub struct Api {
    pub provider: Providers,
    pub longitude: Longitude,
    pub latitude: Latitude,
    #[serde(default)]
    pub timezone: chrono_tz::Tz,
    /// Base URL for the BOM API; overridable so tests can point at a mock server.
    pub bom_base_url: Url,
    /// Base URL for the Open-Meteo API; overridable so tests can point at a mock server.
    pub open_meteo_base_url: Url,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Colours {
    pub background_colour: Colour,
    pub text_colour: Colour,
    pub x_axis_colour: Colour,
    pub y_left_axis_colour: Colour,
    pub y_right_axis_colour: Colour,
    pub actual_temp_colour: Colour,
    pub feels_like_colour: Colour,
    pub rain_colour: Colour,
    pub snow_colour: Colour,
}

/// Detects the system timezone, falling back to UTC and logging a warning
/// distinguishing *why* detection failed — otherwise a misconfigured device
/// silently renders every time in UTC with no indication in its logs.
fn system_timezone() -> chrono_tz::Tz {
    match iana_time_zone::get_timezone() {
        Ok(name) => match name.parse() {
            Ok(tz) => tz,
            Err(_) => {
                crate::logger::warning(format!(
                    "detected system timezone '{name}' is not a recognized IANA identifier; \
                     falling back to UTC. Set `misc.timezone` in config to override."
                ));
                chrono_tz::UTC
            }
        },
        Err(e) => {
            crate::logger::warning(format!(
                "could not detect system timezone ({e}); falling back to UTC. \
                 Set `misc.timezone` in config to override."
            ));
            chrono_tz::UTC
        }
    }
}

// TODO: rename the fields to indicate if it's a path or a name
#[derive(Debug, Deserialize)]
pub struct Misc {
    /// IANA timezone all "local" times are rendered in (e.g. "Australia/Melbourne").
    /// When absent from config, the system timezone is detected once at load
    /// time (UTC as a last resort), so the rest of the code always sees a
    /// concrete timezone.
    #[serde(default = "system_timezone")]
    pub timezone: chrono_tz::Tz,
    pub weather_data_cache_path: PathBuf,
    pub template_path: PathBuf,
    pub generated_svg_name: PathBuf,
    pub generated_png_name: PathBuf,
    #[serde(default = "default_generated_raw_name")]
    pub generated_raw_name: PathBuf,
    pub svg_icons_directory: PathBuf,
    #[serde(default = "default_png_scale_factor")]
    pub png_scale_factor: f32,
}

fn default_generated_raw_name() -> PathBuf {
    "dashboard.raw".into()
}

fn default_png_scale_factor() -> f32 {
    2.0
}

#[nutype(
    validate(greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Deserialize, Clone, Copy)
)]
pub struct Opacity(f32);

#[derive(Debug, Deserialize, Clone)]
pub struct RenderOptions {
    pub temp_unit: TemperatureUnit,
    pub wind_speed_unit: WindSpeedUnit,
    pub language: Language,
    #[serde(default)]
    pub hour_format: HourFormat,
    pub date_format: DateFormat,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    pub use_moon_phase_instead_of_clear_night: bool,
    pub x_axis_always_at_min: bool,
    pub use_gust_instead_of_wind: bool,
    pub prefer_weather_codes: bool,
    /// Gradient fill opacity at 0% precipitation chance. Must be < precipitation_opacity_max.
    pub precipitation_opacity_min: Opacity,
    /// Gradient fill opacity at 100% precipitation chance. Must be > precipitation_opacity_min.
    pub precipitation_opacity_max: Opacity,
}

fn default_time_format() -> String {
    "%T".into()
}

#[derive(Debug, Deserialize)]
pub struct Dev {
    pub disable_weather_api_requests: bool,
    pub disable_png_output: bool,
    #[serde(default)]
    pub disable_raw_7color_output: bool,
    pub enable_debug_logs: bool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct WebServer {
    pub active_hours_start: u8,
    pub active_hours_end: u8,
    pub active_hours_interval_seconds: u32,
}

impl Default for WebServer {
    fn default() -> Self {
        Self {
            active_hours_start: 9,
            active_hours_end: 21,
            active_hours_interval_seconds: 3600,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Network {
    pub proxy: Option<Url>,
}

#[derive(Debug, Deserialize)]
pub struct DashboardSettings {
    #[serde(default)]
    pub network: Network,
    pub release: Release,
    pub api: Api,
    pub colours: Colours,
    pub misc: Misc,
    pub render_options: RenderOptions,
    pub dev: Dev,
    #[serde(default)]
    pub web_server: WebServer,
}

/// Validates cross-field constraints on release settings.
///
/// Returns `Ok(())` if valid, or `Err(message)` describing the conflict.
fn validate_release_cross_fields(
    update_interval_days: UpdateIntervalDays,
    allow_pre_release_version: bool,
) -> Result<(), String> {
    if allow_pre_release_version && update_interval_days.into_inner() == 0 {
        return Err(
            "Configuration validation failed: `allow_pre_release_version` cannot be \
             enabled when `update_interval_days` is 0 (auto-updating is disabled)"
                .to_string(),
        );
    }
    Ok(())
}

/// Which config layer to merge on top of `default.toml`, selected by `RUN_MODE`.
enum ConfigLayer {
    /// `development.toml` + `local.toml` (local dev overrides, not checked into git).
    Development,
    /// `test.toml` only — deterministic, no `development.toml`/`local.toml`.
    Test,
}

/// Dashboard settings.
///
/// # Fields
///
/// * `release` - Release settings.
/// * `api` - API settings.
/// * `colours` - Colour settings.
/// * `misc` - Miscellaneous settings.
/// * `render_options` - Render options.
/// * `dev` - Development/debug settings.
///
/// # Errors
///
/// Returns an error if the configuration cannot be loaded.
///
/// # Panics
///
/// Panics if the configuration file is not found.
impl DashboardSettings {
    /// Loads configuration from config files, the user config, and `APP_*`
    /// environment variables. Called once at startup; the result is passed
    /// down by reference (no global).
    ///
    /// `RUN_MODE=test` selects the same deterministic layer [`Self::load_test_config`]
    /// uses, but — unlike `load_test_config` — still merges the user config file
    /// and `APP_*` environment variables on top of it.
    pub fn load() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        let layer = if run_mode == "test" {
            ConfigLayer::Test
        } else {
            ConfigLayer::Development
        };
        Self::load_from_sources(layer, /* include_user_and_env */ true)
    }

    /// Load configuration for tests: `default.toml` merged with `test.toml` only.
    ///
    /// Deliberately skips the user config file (`~/.config/...`) and `APP_*`
    /// environment variables so results are deterministic regardless of the
    /// invoking shell. Tests mutate the returned settings and pass them
    /// directly into the code under test (see `tests/helpers/test_utils.rs`).
    ///
    /// Stays `pub` rather than `pub(crate)`: `tests/helpers/test_utils.rs`
    /// compiles as a separate crate and needs to call this from the
    /// wiremock/snapshot integration tests that legitimately remain in
    /// `tests/` (see docs/test-suite-review.md).
    pub fn load_test_config() -> Result<Self, ConfigError> {
        Self::load_from_sources(ConfigLayer::Test, /* include_user_and_env */ false)
    }

    /// Shared source-composition pipeline behind [`Self::load`] and
    /// [`Self::load_test_config`], so the two never drift on how a config
    /// layer is merged in — only on whether the user config file and `APP_*`
    /// env are included.
    fn load_from_sources(
        layer: ConfigLayer,
        include_user_and_env: bool,
    ) -> Result<Self, ConfigError> {
        let root = std::env::current_dir().map_err(|e| ConfigError::Message(e.to_string()))?;
        let default_config_path = root.join(CONFIG_DIR).join(DEFAULT_CONFIG_NAME);

        let mut config_builder =
            Config::builder().add_source(File::with_name(default_config_path.to_str().unwrap()));

        if include_user_and_env {
            // user config path is located at ~/.config/pi-inky-weather-epd.toml
            match env::var("HOME") {
                Ok(home_dir) => {
                    let user_config_path = std::path::PathBuf::from(&home_dir)
                        .join(".config")
                        .join(env!("CARGO_PKG_NAME"));
                    config_builder = config_builder.add_source(
                        File::with_name(user_config_path.to_str().unwrap()).required(false),
                    );
                }
                Err(_) => {
                    crate::logger::warning(
                        "HOME environment variable not set; skipping user config file (~/.config/...)",
                    );
                }
            }
        }

        config_builder = match layer {
            ConfigLayer::Test => {
                let test_config_path = root.join(CONFIG_DIR).join("test");
                config_builder
                    .add_source(File::with_name(test_config_path.to_str().unwrap()).required(false))
            }
            ConfigLayer::Development => {
                let development_config_path = root.join(CONFIG_DIR).join("development");
                let local_config_path = root.join(CONFIG_DIR).join("local");
                config_builder
                    .add_source(
                        File::with_name(development_config_path.to_str().unwrap()).required(false),
                    )
                    .add_source(
                        File::with_name(local_config_path.to_str().unwrap()).required(false),
                    )
            }
        };

        if include_user_and_env {
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_API__PROVIDER=open_meteo` would set the `api.provider` key
            // Note: Single underscore _ separates prefix from key, double __ for nesting
            config_builder = config_builder.add_source(
                Environment::with_prefix("APP")
                    .prefix_separator("_") // Separator between prefix and key (APP_api)
                    .separator("__") // Separator for nested keys (api__provider)
                    .try_parsing(true), // Parse values to correct types
            );
        }

        let settings = config_builder.build()?;
        Self::deserialize_and_validate(settings)
    }

    fn deserialize_and_validate(settings: Config) -> Result<Self, ConfigError> {
        let final_settings: Result<DashboardSettings, ConfigError> = settings.try_deserialize();

        // Validate the settings after deserializing
        if let Err(error) = &final_settings {
            return Err(ConfigError::Message(format!(
                "Configuration validation failed: {error:?}"
            )));
        }

        // Cross-field validation: allow_pre_release_version has no effect when
        // update_interval_days = 0 (auto-updating is disabled), so flag it as a
        // misconfiguration to prevent accidental enabling.
        if let Ok(ref s) = final_settings {
            if let Err(msg) = validate_release_cross_fields(
                s.release.update_interval_days,
                s.release.allow_pre_release_version,
            ) {
                return Err(ConfigError::Message(msg));
            }

            let omin = s.render_options.precipitation_opacity_min.into_inner();
            let omax = s.render_options.precipitation_opacity_max.into_inner();
            if omin >= omax {
                return Err(ConfigError::Message(format!(
                    "precipitation_opacity_min ({omin}) must be less than precipitation_opacity_max ({omax})"
                )));
            }
        }

        final_settings
    }

    /// Print configuration settings in a structured, hierarchical format
    pub fn print_config(&self) {
        use crate::logger;

        logger::section("Configuration loaded");

        logger::config_group("Network Settings");
        logger::kvp("Proxy configured", self.network.proxy.is_some());

        // API Settings
        logger::config_group("API Settings");
        logger::kvp("Provider", format!("{}", self.api.provider));
        logger::kvp("API Timezone", self.api.timezone);
        logger::kvp(
            "Location",
            format!(
                "lat: {}, lon: {}",
                self.api.latitude.into_inner(),
                self.api.longitude.into_inner()
            ),
        );

        // Render Options
        logger::config_group("Render Options");
        logger::kvp(
            "Temperature Unit",
            format!("{}", self.render_options.temp_unit),
        );
        logger::kvp(
            "Wind Speed Unit",
            format!("{}", self.render_options.wind_speed_unit),
        );
        logger::kvp("Language", format!("{}", self.render_options.language));
        logger::kvp(
            "Hour Format",
            format!("{}", self.render_options.hour_format),
        );
        logger::kvp("Date Format", &self.render_options.date_format);
        logger::kvp("Time Format", &self.render_options.time_format);
        logger::kvp(
            "Use Moon Phase",
            self.render_options.use_moon_phase_instead_of_clear_night,
        );
        logger::kvp(
            "X-Axis Always at Min",
            self.render_options.x_axis_always_at_min,
        );
        logger::kvp(
            "Use Gust Instead of Wind",
            self.render_options.use_gust_instead_of_wind,
        );
        logger::kvp(
            "Prefer Weather Codes",
            self.render_options.prefer_weather_codes,
        );

        // Colours
        logger::config_group("Display Colours");
        logger::kvp("Background", &self.colours.background_colour);
        logger::kvp("Text", &self.colours.text_colour);
        logger::kvp("X-Axis", &self.colours.x_axis_colour);
        logger::kvp("Y-Left Axis (Temp)", &self.colours.y_left_axis_colour);
        logger::kvp("Y-Right Axis (Rain)", &self.colours.y_right_axis_colour);
        logger::kvp("Actual Temp", &self.colours.actual_temp_colour);
        logger::kvp("Feels Like", &self.colours.feels_like_colour);
        logger::kvp("Rain", &self.colours.rain_colour);
        logger::kvp("Snow", &self.colours.snow_colour);

        // File Paths
        logger::config_group("File Paths");
        logger::kvp("Cache Path", self.misc.weather_data_cache_path.display());
        logger::kvp("Template", self.misc.template_path.display());
        logger::kvp("PNG Scale factor", self.misc.png_scale_factor);
        logger::kvp("Output SVG", self.misc.generated_svg_name.display());
        logger::kvp("Output PNG", self.misc.generated_png_name.display());
        logger::kvp("Output RAW", self.misc.generated_raw_name.display());
        logger::kvp("Icons Directory", self.misc.svg_icons_directory.display());

        // Release/Update Settings
        logger::config_group("Update Settings");
        logger::kvp("Update Interval (days)", self.release.update_interval_days);
        logger::kvp("Allow Pre-release", self.release.allow_pre_release_version);

        // Dev Flags
        logger::config_group("Dev Flags");
        logger::kvp(
            "Disable API Requests",
            self.dev.disable_weather_api_requests,
        );
        logger::kvp("Disable PNG Output", self.dev.disable_png_output);
        logger::kvp(
            "Disable RAW 7color Output",
            self.dev.disable_raw_7color_output,
        );
        logger::kvp("Enable Debug Logs", self.dev.enable_debug_logs);

        // Cross-configuration warnings
        if self.api.provider == Providers::Bom && self.render_options.prefer_weather_codes {
            logger::warning(
                "`prefer_weather_codes = true` has no effect with the BOM provider — \
                BOM does not supply WMO weather codes, so icon selection will always \
                use derived logic",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DashboardSettings, UpdateIntervalDays, validate_release_cross_fields};
    use config::{Config, Environment, File, FileFormat};

    #[test]
    fn upstream_configs_default_missing_local_settings() {
        let config = Config::builder()
            .add_source(File::from_str(
                include_str!("../../config/default.toml"),
                FileFormat::Toml,
            ))
            .build()
            .unwrap();
        let mut json: serde_json::Value = config.try_deserialize().unwrap();
        json.as_object_mut().unwrap().remove("network");
        json.as_object_mut().unwrap().remove("web_server");
        json["api"].as_object_mut().unwrap().remove("timezone");
        json["misc"]
            .as_object_mut()
            .unwrap()
            .remove("png_scale_factor");
        json["misc"]
            .as_object_mut()
            .unwrap()
            .remove("generated_raw_name");
        json["render_options"]
            .as_object_mut()
            .unwrap()
            .remove("time_format");
        json["dev"]
            .as_object_mut()
            .unwrap()
            .remove("disable_raw_7color_output");
        let settings: DashboardSettings = serde_json::from_value(json).unwrap();
        assert!(settings.network.proxy.is_none());
        assert_eq!(settings.api.timezone, chrono_tz::UTC);
        assert_eq!(settings.misc.png_scale_factor, 2.0);
        assert_eq!(
            settings.misc.generated_raw_name,
            std::path::Path::new("dashboard.raw")
        );
        assert_eq!(settings.render_options.time_format, "%T");
        assert!(!settings.dev.disable_raw_7color_output);
        assert_eq!(settings.web_server.active_hours_start, 9);
        assert_eq!(settings.web_server.active_hours_end, 21);
        assert_eq!(settings.web_server.active_hours_interval_seconds, 3600);
    }

    #[test]
    fn network_proxy_is_optional_in_existing_configs() {
        let default = include_str!("../../config/default.toml");
        let settings: DashboardSettings = Config::builder()
            .add_source(File::from_str(
                &default[default.find("[release]").unwrap()..],
                FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        assert!(settings.network.proxy.is_none());
    }

    #[test]
    fn network_proxy_can_be_overridden_by_environment() {
        let proxy = "socks5h://user:password@localhost:1081";
        let settings: DashboardSettings = Config::builder()
            .add_source(File::from_str(
                include_str!("../../config/default.toml"),
                FileFormat::Toml,
            ))
            .add_source(File::from_str(
                "[network]\nproxy = 'socks5://127.0.0.1:1080'",
                FileFormat::Toml,
            ))
            .add_source(
                Environment::with_prefix("APP")
                    .prefix_separator("_")
                    .separator("__")
                    .source(Some(std::collections::HashMap::from([(
                        "APP_NETWORK__PROXY".into(),
                        proxy.into(),
                    )]))),
            )
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        assert_eq!(settings.network.proxy.unwrap().as_str(), proxy);
    }

    #[test]
    fn allow_pre_release_with_zero_interval_is_rejected() {
        let result = validate_release_cross_fields(UpdateIntervalDays::try_new(0).unwrap(), true);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("`allow_pre_release_version`"));
        assert!(msg.contains("`update_interval_days` is 0"));
    }

    #[test]
    fn allow_pre_release_with_nonzero_interval_is_accepted() {
        assert!(
            validate_release_cross_fields(UpdateIntervalDays::try_new(7).unwrap(), true).is_ok()
        );
    }

    #[test]
    fn disallow_pre_release_with_zero_interval_is_accepted() {
        assert!(
            validate_release_cross_fields(UpdateIntervalDays::try_new(0).unwrap(), false).is_ok()
        );
    }

    #[test]
    fn disallow_pre_release_with_nonzero_interval_is_accepted() {
        assert!(
            validate_release_cross_fields(UpdateIntervalDays::try_new(7).unwrap(), false).is_ok()
        );
    }

    #[test]
    fn hour_format_deserializes_documented_values() {
        assert_eq!(
            serde_json::from_str::<super::HourFormat>("\"12h\"").unwrap(),
            super::HourFormat::TwelveHour
        );
        assert_eq!(
            serde_json::from_str::<super::HourFormat>("\"24h\"").unwrap(),
            super::HourFormat::TwentyFour
        );
    }
}

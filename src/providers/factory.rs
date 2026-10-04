use crate::{
    configs::settings::{DashboardSettings, Providers},
    providers::{WeatherProvider, bom::BomProvider, open_meteo::OpenMeteoProvider},
};

pub fn create_provider(settings: &DashboardSettings) -> anyhow::Result<Box<dyn WeatherProvider>> {
    let cache_path = settings.misc.weather_data_cache_path.clone();

    match settings.api.provider {
        Providers::Bom => Ok(Box::new(BomProvider::new(
            cache_path,
            settings.network.proxy.as_ref(),
        ))),
        Providers::OpenMeteo => Ok(Box::new(OpenMeteoProvider::new(
            cache_path,
            settings.network.proxy.as_ref(),
        ))),
    }
}

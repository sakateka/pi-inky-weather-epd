mod helpers;

use helpers::test_utils;
use std::fs;

/// Validates that the base SVG template file is valid
///
/// This test ensures the template file can be parsed as valid SVG.
/// It doesn't test rendering - see tests/snapshot_test.rs for E2E tests.
#[test]
fn base_template_svg_ok() {
    let settings = test_utils::test_settings(|_| {});
    let svg_content = fs::read_to_string(&settings.misc.template_path)
        .expect("Failed to read the base template SVG file");
    let svg_tree = usvg::Tree::from_str(&svg_content, &usvg::Options::default());

    assert!(
        svg_tree.is_ok(),
        "The base template file is not a valid SVG"
    );
}

#[tokio::test]
async fn waveshare_template_renders_png_and_raw_in_every_language() {
    use pi_inky_weather_epd::{
        FixedClock,
        i18n::{Language, TranslationKey, translate},
        utils::{convert_png_bytes_to_raw_7color, convert_svg_to_png_bytes},
        weather_dashboard::generate_dashboard_svg_string,
    };
    for language in [
        Language::En,
        Language::Fr,
        Language::De,
        Language::Es,
        Language::Ja,
    ] {
        let server = helpers::wiremock_setup::setup_open_meteo_mock(
            "tests/fixtures/open_meteo_hourly_forecast.json",
            "tests/fixtures/open_meteo_daily_forecast.json",
        )
        .await;
        let mut settings = test_utils::open_meteo_settings(&server.uri());
        settings.misc.template_path = "dashboard-template-5.65f.svg".into();
        settings.render_options.language = language;
        tokio::task::spawn_blocking(move || {
            let clock = FixedClock::from_rfc3339("2025-10-25T01:00:00Z").unwrap();
            let svg = generate_dashboard_svg_string(&settings, &clock).unwrap();
            assert!(svg.contains(translate(TranslationKey::Now, language)));
            assert!(svg.contains("12:00:00"));
            let png = convert_svg_to_png_bytes(&svg, 1.0).unwrap();
            let image = image::load_from_memory(&png).unwrap();
            assert_eq!((image.width(), image.height()), (600, 448));
            assert_eq!(
                convert_png_bytes_to_raw_7color(&png).unwrap().len(),
                600 * 448 / 2
            );
            fs::create_dir_all("tests/output").unwrap();
            fs::write(format!("tests/output/waveshare-{language}.png"), png).unwrap();
        })
        .await
        .unwrap();
    }
}

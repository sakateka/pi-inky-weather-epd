use crate::errors::GeohashError;
use crate::logger;
use anyhow::Error;
use anyhow::Result;
use chrono::Local;
use chrono::TimeZone;
use chrono::{DateTime, NaiveDateTime};
use resvg::tiny_skia;
use resvg::usvg;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use usvg::fontdb;

/// Converts an SVG file to a PNG file.
///
/// # Arguments
///
/// * `input_path` - Path to the input SVG file.
/// * `output_path` - Path to save the output PNG file.
/// * `scale_factor` - The scale factor to apply to the SVG.
///
/// # Returns
///
/// * `Result<(), Error>` - Ok(()) if successful, or an error message.
pub fn convert_svg_to_png(
    input_path: &PathBuf,
    output_path: &PathBuf,
    scale_factor: f32,
) -> Result<(), Error> {
    // Read the SVG file
    let svg_data = fs::read_to_string(input_path)
        .map_err(|e| Error::msg(format!("Failed to read SVG file: {e}")))?;

    let png_bytes = convert_svg_to_png_bytes(&svg_data, scale_factor)?;

    // Save the PNG file
    fs::write(output_path, &png_bytes)
        .map_err(|e| Error::msg(format!("Failed to save PNG: {e}")))?;

    Ok(())
}

/// Converts SVG string to PNG bytes in memory.
///
/// # Arguments
///
/// * `svg_data` - SVG content as string
/// * `scale_factor` - The scale factor to apply to the SVG
///
/// # Returns
///
/// * `Result<Vec<u8>, Error>` - PNG image data as bytes
pub fn convert_svg_to_png_bytes(svg_data: &str, scale_factor: f32) -> Result<Vec<u8>, Error> {
    let mut font_db = fontdb::Database::new();
    load_fonts(&mut font_db);

    // Parse the SVG
    let opts = usvg::Options {
        fontdb: font_db.into(),
        ..Default::default()
    };

    let tree = usvg::Tree::from_str(svg_data, &opts)
        .map_err(|e| Error::msg(format!("Failed to parse SVG: {e}")))?;

    // Create a higher resolution canvas
    let pixmap_size = tree.size().to_int_size();
    let width = (pixmap_size.width() as f32 * scale_factor) as u32;
    let height = (pixmap_size.height() as f32 * scale_factor) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| Error::msg("Failed to create pixmap"))?;

    // Create a transform that scales the SVG
    let transform = tiny_skia::Transform::from_scale(scale_factor, scale_factor);

    // Render SVG onto the canvas with scaling
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Encode PNG to bytes
    pixmap
        .encode_png()
        .map_err(|e| Error::msg(format!("Failed to encode PNG: {e}")))
}

/// 7-color e-ink display palette (RGB values)
/// Colors: Black, White, Green, Blue, Red, Yellow, Orange, Purple
const PALETTE_7COLOR: [[u8; 3]; 8] = [
    [0, 0, 0],       // Black
    [255, 255, 255], // White
    [67, 138, 28],   // Green
    [100, 64, 255],  // Blue
    [191, 0, 0],     // Red
    [255, 243, 56],  // Yellow
    [232, 126, 0],   // Orange
    [194, 164, 244], // Purple
];

/// Finds the closest palette color index for a given RGB color using Euclidean distance.
///
/// # Arguments
///
/// * `color` - RGB color as [r, g, b] array
///
/// # Returns
///
/// * `u8` - Index of the closest palette color (0-7)
fn depalette(color: [u8; 3]) -> u8 {
    let mut min_diff = i32::MAX;
    let mut best_index = 0u8;

    for (index, palette_color) in PALETTE_7COLOR.iter().enumerate() {
        let diff_r = color[0] as i32 - palette_color[0] as i32;
        let diff_g = color[1] as i32 - palette_color[1] as i32;
        let diff_b = color[2] as i32 - palette_color[2] as i32;
        let diff = diff_r * diff_r + diff_g * diff_g + diff_b * diff_b;

        if diff < min_diff {
            min_diff = diff;
            best_index = index as u8;
        }
    }

    best_index
}

/// Helper function to convert RGB image to raw 7-color format.
///
/// # Arguments
///
/// * `rgb_img` - RGB8 image
///
/// # Returns
///
/// * `Vec<u8>` - Raw 4-bit color data
fn rgb_to_raw_7color(rgb_img: &image::RgbImage) -> Vec<u8> {
    let (width, height) = rgb_img.dimensions();

    // Calculate output buffer size (2 pixels per byte due to 4-bit packing)
    let total_pixels = (width * height) as usize;
    let output_size = total_pixels.div_ceil(2);
    let mut output_buffer = Vec::with_capacity(output_size);

    // Process pixels row by row, in pairs
    for y in 0..height {
        for i in 0..(width / 2) {
            let x1 = i * 2;
            let x2 = i * 2 + 1;

            // Get first pixel (even x position)
            let pixel1 = rgb_img.get_pixel(x1, y);
            let color1 = [pixel1[0], pixel1[1], pixel1[2]];
            let c1 = depalette(color1);

            // Get second pixel (odd x position)
            let pixel2 = rgb_img.get_pixel(x2, y);
            let color2 = [pixel2[0], pixel2[1], pixel2[2]];
            let c2 = depalette(color2);

            // Pack two 4-bit indices into one byte
            // c1 goes to high nibble, c2 goes to low nibble
            let packed_byte = c2 | (c1 << 4);
            output_buffer.push(packed_byte);
        }

        // Handle odd width - add padding pixel if necessary
        if width % 2 == 1 {
            let x = width - 1;
            let pixel = rgb_img.get_pixel(x, y);
            let color = [pixel[0], pixel[1], pixel[2]];
            let c = depalette(color);
            // Last pixel in high nibble, low nibble is 0 (black)
            let packed_byte = c << 4;
            output_buffer.push(packed_byte);
        }
    }

    output_buffer
}

/// Converts a PNG image to raw 7-color format with 4-bit nibble packing.
///
/// Each pixel is mapped to the closest color in the 7-color palette,
/// then packed as 4-bit values (2 pixels per byte).
///
/// # Arguments
///
/// * `input_path` - Path to the input PNG file
/// * `output_path` - Path to save the output raw file
///
/// # Returns
///
/// * `Result<(), Error>` - Ok(()) if successful, or an error message
pub fn convert_png_to_raw_7color(input_path: &PathBuf, output_path: &PathBuf) -> Result<(), Error> {
    // Load the PNG image
    let img =
        image::open(input_path).map_err(|e| Error::msg(format!("Failed to open PNG file: {e}")))?;

    // Convert to RGB8 format
    let rgb_img = img.to_rgb8();
    let output_buffer = rgb_to_raw_7color(&rgb_img);

    // Write the packed data to the output file
    fs::write(output_path, &output_buffer)
        .map_err(|e| Error::msg(format!("Failed to write raw file: {e}")))?;

    Ok(())
}

/// Converts PNG bytes to raw 7-color format with 4-bit nibble packing.
///
/// Each pixel is mapped to the closest color in the 7-color palette,
/// then packed as 4-bit values (2 pixels per byte).
///
/// # Arguments
///
/// * `png_data` - PNG image data as bytes
///
/// # Returns
///
/// * `Result<Vec<u8>, Error>` - Raw 4-bit color data
pub fn convert_png_bytes_to_raw_7color(png_data: &[u8]) -> Result<Vec<u8>, Error> {
    // Load the PNG image from bytes
    let img = image::load_from_memory(png_data)
        .map_err(|e| Error::msg(format!("Failed to load PNG from memory: {e}")))?;

    // Convert to RGB8 format
    let rgb_img = img.to_rgb8();
    Ok(rgb_to_raw_7color(&rgb_img))
}

/// Loads fonts into the provided font database.
///
/// # Arguments
///
/// * `font_db` - A mutable reference to a `fontdb::Database` to load fonts into.
fn load_fonts(font_db: &mut fontdb::Database) {
    font_db.load_system_fonts();

    // print current path
    let current_path = std::env::current_dir().unwrap();

    let font_files = [
        "static/fonts/Roboto-VariableFont_wdth,wght.ttf",
        "static/fonts/Roboto-Italic-VariableFont_wdth,wght.ttf",
        "static/fonts/Roboto-Regular-Dashed.ttf",
    ];

    for file in &font_files {
        match font_db.load_font_file(current_path.join(file)) {
            Ok(_) => {}
            Err(e) => logger::warning(format!("Failed to load font file: {e}")),
        }
    }
}

/// Calculates the total value between two dates from a dataset.
///
/// # Arguments
///
/// * `data` - A slice of data items.
/// * `start_date` - The start date as `DateTime<TZ>`.
/// * `end_date` - The end date as `DateTime<TZ>`.
/// * `get_value` - A function to extract the value from a data item.
/// * `get_time` - A function to extract the time from a data item.
///
/// # Returns
///
/// * `V` - The total value between the specified dates.
pub fn get_total_between_dates<T, V, TZ: TimeZone>(
    data: &[T],
    start_date: &DateTime<TZ>,
    end_date: &DateTime<TZ>,
    get_value: impl Fn(&T) -> V,
    get_time: impl Fn(&T) -> DateTime<TZ>,
) -> V
where
    V: std::iter::Sum + Default,
{
    data.iter()
        .filter_map(|item| {
            let item_date = &get_time(item);
            if item_date >= start_date && item_date < end_date {
                Some(get_value(item))
            } else {
                None
            }
        })
        .sum()
}

/// Finds the maximum value between two dates from a dataset.
///
/// # Arguments
///
/// * `data` - A slice of data items.
/// * `start_date` - The start date as `DateTime<TZ>`.
/// * `end_date` - The end date as `DateTime<TZ>`, not inclusive.
/// * `get_value` - A function to extract the value from a data item.
/// * `get_time` - A function to extract the time from a data item.
///
/// # Returns
///
/// * `V` - The maximum value between the specified dates.
pub fn find_max_item_between_dates<T, V, TZ: TimeZone>(
    data: &[T],
    start_date: &DateTime<TZ>,
    end_date: &DateTime<TZ>,
    get_value: impl Fn(&T) -> V,
    get_time: impl Fn(&T) -> DateTime<TZ>,
) -> V
where
    V: PartialOrd + Copy + Default,
{
    // Use V::default() as the initial value for finding the maximum, it should be fine for numeric types here since they are all positive
    data.iter()
        .filter_map(|item| {
            let date = &get_time(item);
            if date >= start_date && date < end_date {
                Some(get_value(item))
            } else {
                None
            }
        })
        .fold(V::default(), |acc, x| if x > acc { x } else { acc })
}

/// Deserializes an optional NaiveDateTime from a string.
///
/// # Arguments
///
/// * `deserializer` - The deserializer to use.
///
/// # Returns
///
/// * `Result<Option<NaiveDateTime>, D::Error>` - The deserialized `NaiveDateTime` or an error.
pub fn deserialize_optional_naive_date<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    if let Some(date_str) = opt {
        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%dT%H:%M:%SZ")
            .map(|dt| Some(Local.from_utc_datetime(&dt).naive_local()))
            .map_err(serde::de::Error::custom)
    } else {
        Ok(None)
    }
}

/// Deserializes a NaiveDateTime from a string.
///
/// # Arguments
///
/// * `s` - The deserializer to use.
///
/// # Returns
///
/// * `Result<NaiveDateTime, S::Error>` - The deserialized `NaiveDateTime` or an error.
pub fn deserialize_naive_date<'de, S>(s: S) -> Result<NaiveDateTime, S::Error>
where
    S: serde::de::Deserializer<'de>,
{
    let date_str: &str = serde::Deserialize::deserialize(s)?;
    NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%SZ")
        .map(|dt| Local.from_utc_datetime(&dt).naive_local())
        .map_err(serde::de::Error::custom)
}

// Below code was adopted from Geohash crate
// https://github.com/georust/geohash/blob/main/src/core.rs

// the alphabet for the base32 encoding used in geohashing
#[rustfmt::skip]
const BASE32_CODES: [char; 32] = [
    '0', '1', '2', '3', '4', '5', '6', '7',
    '8', '9', 'b', 'c', 'd', 'e', 'f', 'g',
    'h', 'j', 'k', 'm', 'n', 'p', 'q', 'r',
    's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

// bit shifting functions used in encoding and decoding

// spread takes a u32 and deposits its bits into the evenbit positions of a u64
#[inline]
fn spread(x: u32) -> u64 {
    let mut new_x = x as u64;
    new_x = (new_x | (new_x << 16)) & 0x0000ffff0000ffff;
    new_x = (new_x | (new_x << 8)) & 0x00ff00ff00ff00ff;
    new_x = (new_x | (new_x << 4)) & 0x0f0f0f0f0f0f0f0f;
    new_x = (new_x | (new_x << 2)) & 0x3333333333333333;
    new_x = (new_x | (new_x << 1)) & 0x5555555555555555;

    new_x
}

// spreads the inputs, then shifts the y input and does a bitwise or to fill the remaining bits in x
#[inline]
fn interleave(x: u32, y: u32) -> u64 {
    spread(x) | (spread(y) << 1)
}

/// Encode a coordinate to a geohash with length `len`.
///
/// # Arguments
///
/// * `lon_x` - The longitude (x coordinate) in degrees, must be in range [-180, 180]
/// * `lat_y` - The latitude (y coordinate) in degrees, must be in range [-90, 90]
/// * `len` - The desired length of the geohash string (1-12)
///
/// # Examples
///
/// Encoding a coordinate to a length five geohash:
///
/// ```ignore
/// let geohash_string = encode(-120.6623, 35.3003, 5).expect("Invalid coordinate");
/// assert_eq!(geohash_string, "9q60y");
/// ```
///
/// Encoding a coordinate to a length ten geohash:
///
/// ```ignore
/// let geohash_string = encode(-120.6623, 35.3003, 10).expect("Invalid coordinate");
/// assert_eq!(geohash_string, "9q60y60rhs");
/// ```
pub fn encode(lon_x: f64, lat_y: f64, len: usize) -> Result<String, GeohashError> {
    let max_lat = 90f64;
    let min_lat = -90f64;
    let max_lon = 180f64;
    let min_lon = -180f64;

    if !(min_lon..=max_lon).contains(&lon_x) || !(min_lat..=max_lat).contains(&lat_y) {
        return Err(GeohashError::InvalidCoordinateRange(lon_x, lat_y));
    }

    if !(1..=12).contains(&len) {
        return Err(GeohashError::InvalidLength(len));
    }

    // divides the latitude by 180, then adds 1.5 to give a value between 1 and 2
    // then we take the first 32 bits of the significand as a u32
    let lat32 = ((lat_y * 0.005555555555555556 + 1.5).to_bits() >> 20) as u32;
    // same as latitude, but a division by 360 instead of 180
    let lon32 = ((lon_x * 0.002777777777777778 + 1.5).to_bits() >> 20) as u32;

    let mut interleaved_int = interleave(lat32, lon32);

    let mut out = String::with_capacity(len);
    // loop through and take the first 5 bits of the interleaved value ech iteration
    for _ in 0..len {
        // shifts so that the high 5 bits are now the low five bits, then masks to get their value
        let code = (interleaved_int >> 59) as usize & (0x1f);
        // uses that value to index into the array of base32 codes
        out.push(BASE32_CODES[code]);
        // shifts the interleaved bits left by 5, so we get the next 5 bits on the next iteration
        interleaved_int <<= 5;
    }
    Ok(out)
}

// Finish Geohash crate code

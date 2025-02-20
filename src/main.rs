//! A tracing machine for Rust

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::restriction,
    clippy::style,
    clippy::suspicious
)]
#![expect(
    clippy::print_stdout,
    clippy::implicit_return,
    clippy::unseparated_literal_suffix,
    clippy::multiple_crate_versions,
    clippy::blanket_clippy_restriction_lints,
    reason = "conflicting lints"
)]

mod impls;
mod models;

use crate::models::DrawingApp;
use core::str::FromStr as _;
use device_query::{DeviceQuery as _, DeviceState};
use enigo::{Enigo, Settings};
use image::imageops::FilterType::Lanczos3;
use image::{DynamicImage, ImageBuffer, Luma};
use native_dialog::FileDialog;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut app = match anyhow::Ok(DrawingApp {
        enigo: match Enigo::new(&Settings::default()) {
            Ok(it) => it,
            Err(err) => return Err(err.into()),
        },
        device_state: DeviceState::new(),
    }) {
        Ok(it) => it,
        Err(err) => return Err(err),
    };

    println!("Please select an image file");
    let image_path: String = FileDialog::new()
        .add_filter("Image Files", &["png", "jpg", "jpeg"])
        .show_open_single_file()
        .ok()
        .unwrap_or(Some(match PathBuf::from_str("fuck.txt") {
            Ok(it) => it,
            Err(err) => return Err(err.into()),
        }))
        .map(|path| path.to_string_lossy().into_owned())
        .map_or_else(
            || "Couldn't select image file. Exiting...".to_owned(),
            |path| path,
        );

    let binary_img_result = {
        let img_path_wrapper: &str = &image_path;
        let img = match image::open(img_path_wrapper) {
            Ok(it) => it,
            Err(err) => return Err(err.into()),
        };
        let gray_img = img.to_luma8();
        let mut binary_image = ImageBuffer::new(gray_img.width(), gray_img.height());

        for (x, y, px) in gray_img.enumerate_pixels() {
            let pixel_val = if px.0[0] > 128 { 255 } else { 0 };
            binary_image.put_pixel(x, y, Luma([pixel_val]));
        }

        Ok(binary_image)
    };

    let bw_img = match binary_img_result {
        Ok(it) => it,
        Err(err) => return Err(err),
    };

    println!("Move your cursor to select the region where you want to draw.");
    let (start_pos, end_pos) = app.capture_screen_region();
    let scaled_img = {
        let img: &ImageBuffer<Luma<u8>, Vec<u8>> = &bw_img;
        let width = (end_pos.0.checked_sub(start_pos.0).unwrap_or(0i32)).unsigned_abs();
        let height = (end_pos.1.checked_sub(start_pos.1).unwrap_or(0i32)).unsigned_abs();

        DynamicImage::ImageLuma8(img.clone())
            .resize(width, height, Lanczos3)
            .to_luma8()
    };

    println!("Ready to draw! Press 'D' to start drawing or 'Q' to quit");
    loop {
        let keys = app.device_state.get_keys();
        match keys {
            key if key.contains(&device_query::Keycode::D) => {
                match app.trace(&scaled_img, start_pos, 1) {
                    Ok(it) => it,
                    Err(err) => return Err(err),
                }

                break;
            }
            key if key.contains(&device_query::Keycode::Q) => {
                println!("Exiting...");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

use crate::choices::{get_speed, get_step, ImageProcessingMethod};
use crate::drawing::components::find_connected_components_optimized;
use crate::utils::geometry::Point;
use crate::{
    choices::drawing::{DrawingAccuracy, DrawingSpeed, LineOrder, RegionPickMode, ScalingMode},
    image_processing::{ImageProcessor, ImageScaler},
};
use device_query::{DeviceQuery, DeviceState, Keycode};
use enigo::{Enigo, MouseButton, MouseControllable};
use image::{ImageBuffer, Luma};
use indicatif::{ProgressBar, ProgressStyle};
use native_dialog::FileDialog;
use rand::rng;
use rand::seq::SliceRandom;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::collections::HashSet;
use std::thread;
use std::time::Duration;

pub struct DrawingApp {
    enigo: Enigo,
    device_state: DeviceState,
}

impl DrawingApp {
    pub fn new() -> Self {
        DrawingApp {
            enigo: Enigo::new(),
            device_state: DeviceState::new(),
        }
    }

    pub fn run() {
        let mut app = DrawingApp::new();
        app.execute();
    }

    fn select_image() -> Option<String> {
        FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg", "gif"])
            .show_open_single_file()
            .ok()?
            .map(|path| path.to_string_lossy().into_owned())
    }

    fn select_scaling_mode(&self) -> ScalingMode {
        ScalingMode::choice("Please select a scaling method").expect("Failed to get user input")
    }

    fn capture_screen_region(&self) -> ((i32, i32), (i32, i32)) {
        let capture_method = RegionPickMode::choice("How would you like to select the region?")
            .expect("Failed to get user input");

        let (start_pos, end_pos) = match capture_method {
            RegionPickMode::Manual => {
                let tlx = inquire::prompt_u32("Please input the X value of the top left corner")
                    .expect("Failed to get user input") as i32;
                let tly = inquire::prompt_u32("Please input the Y value of the top left corner")
                    .expect("Failed to get user input") as i32;
                let brx = inquire::prompt_u32("Please input the X value of the bottom right corner")
                    .expect("Failed to get user input") as i32;
                let bry = inquire::prompt_u32("Please input the Y value of the bottom right corner")
                    .expect("Failed to get user input") as i32;

                return ((tlx, tly), (brx, bry));
            }
            RegionPickMode::Interactive => {
                println!("Press 'S' to start selecting region");
                self.wait_for_key(Keycode::S);

                let start = self.device_state.get_mouse().coords;
                println!("Start position captured: ({}, {})", start.0, start.1);
                println!("Move to end position and press 'E'");

                self.wait_for_key(Keycode::E);
                let end = self.device_state.get_mouse().coords;
                println!("End position captured: ({}, {})", end.0, end.1);

                (start, end)
            }
        };

        (start_pos, end_pos)
    }

    fn wait_for_key(&self, target_key: Keycode) {
        loop {
            let keys = self.device_state.get_keys();
            if keys.contains(&target_key) {
                self.wait_for_key_release(target_key);
                break;
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_key_release(&self, target_key: Keycode) {
        while self.device_state.get_keys().contains(&target_key) {
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn get_black_pixels_adaptive(
        img: &ImageBuffer<Luma<u16>, Vec<u16>>,
        step: i32,
    ) -> HashSet<Point> {
        let coordinates: Vec<(u32, u32)> = (0..img.height())
            .step_by(step as usize)
            .flat_map(|y| (0..img.width()).step_by(step as usize).map(move |x| (x, y)))
            .collect();

        let black_pixels: Vec<Point> = coordinates
            .par_iter()
            .filter_map(|&(x, y)| {
                if img.get_pixel(x, y)[0] == 0 {
                    Some(Point::new(x as i32, y as i32))
                } else {
                    None
                }
            })
            .collect();

        black_pixels.into_iter().collect()
    }

    fn draw_image_optimized(
        &mut self,
        img: &ImageBuffer<Luma<u16>, Vec<u16>>,
        start_pos: (i32, i32),
        drawing_speed: Duration,
        step: i32,
        line_order: LineOrder,
    ) {
        println!("Drawing will start in 3 seconds. Keep your cursor still!");
        thread::sleep(Duration::from_secs(3));

        let black_pixels = Self::get_black_pixels_adaptive(img, step);
        println!("Found {} black pixels to draw", black_pixels.len());

        if black_pixels.is_empty() {
            println!("No black pixels found to draw!");
            return;
        }

        let mut lines = find_connected_components_optimized(black_pixels, 3);
        println!("Generated {} drawing paths", lines.len());
        let progress_style = ProgressStyle::default_bar()
            .template("{wide_bar} {pos}/{len} ({eta})")
            .expect("Invalid progress style template")
            .progress_chars("=>=");
        let pb = ProgressBar::new(lines.len() as u64);
        pb.set_style(progress_style);

        let total_lines = lines.len();
        let mut rng = rng();

        if line_order == LineOrder::Shuffled {
            lines.shuffle(&mut rng);
        }

        for line in lines.iter() {
            if line.len() < 2 {
                pb.inc(1);
                continue;
            }

            let keys = self.device_state.get_keys();

            if keys.contains(&Keycode::Q) {
                pb.finish_with_message("Cancelled");
                break;
            }

            let abs_start_x = start_pos.0 + line[0].x;
            let abs_start_y = start_pos.1 + line[0].y;

            self.enigo.mouse_move_to(abs_start_x, abs_start_y);
            thread::sleep(drawing_speed);

            self.enigo.mouse_down(MouseButton::Left);

            for points_chunk in line.windows(2) {
                let current = points_chunk[0];
                let next = points_chunk[1];

                let distance = current.distance_squared(&next);
                if distance > 1 {
                    let steps = (distance as f64).sqrt() as i32;
                    for step in 1..=steps {
                        let t = step as f64 / steps as f64;
                        let interp_x = current.x as f64 + t * (next.x - current.x) as f64;
                        let interp_y = current.y as f64 + t * (next.y - current.y) as f64;

                        let abs_x = start_pos.0 + interp_x as i32;
                        let abs_y = start_pos.1 + interp_y as i32;
                        self.enigo.mouse_move_to(abs_x, abs_y);

                        thread::sleep(drawing_speed);
                    }
                } else {
                    let abs_x = start_pos.0 + next.x;
                    let abs_y = start_pos.1 + next.y;
                    self.enigo.mouse_move_to(abs_x, abs_y);

                    if !drawing_speed.is_zero() {
                        thread::sleep(drawing_speed);
                    }
                }
            }

            self.enigo.mouse_up(MouseButton::Left);
            thread::sleep(drawing_speed);
            pb.inc(1);
        }

        pb.finish_with_message(format!(
            "Drawing completed! Drew {} paths with {} total points",
            total_lines,
            lines.iter().map(|l| l.len()).sum::<usize>()
        ));
    }

    fn wait_for_drawing_command(
        &mut self,
        scaled_img: &ImageBuffer<Luma<u16>, Vec<u16>>,
        start_pos: (i32, i32),
        drawing_speed: Duration,
        step: i32,
        line_order: LineOrder,
    ) {
        loop {
            let keys = self.device_state.get_keys();
            if keys.contains(&Keycode::D) {
                self.draw_image_optimized(scaled_img, start_pos, drawing_speed, step, line_order);
                break;
            } else if keys.contains(&Keycode::Q) {
                println!("Quitting!");
                break;
            }

            thread::sleep(drawing_speed);
        }
    }

    fn execute(&mut self) {
        println!("Please select an image file");
        let image_path = match Self::select_image() {
            Some(path) => path,
            None => {
                println!("No image selected. Exiting...");
                return;
            }
        };

        let processing_method =
            ImageProcessingMethod::choice("Please select a method for processing the image")
                .expect("Failed to get user input");
        let bw_img = match ImageProcessor::process_image(&image_path, processing_method) {
            Ok(img) => img,
            Err(e) => {
                println!("Error processing image: {}", e);
                return;
            }
        };

        println!("Image processed successfully!");

        let scaling_mode = self.select_scaling_mode();
        let step = get_step(
            DrawingAccuracy::choice("Please select a desired accuracy for the drawing")
                .expect("Failed to get user input"),
        );
        let drawing_speed = get_speed(
            DrawingSpeed::choice("How fast should the image be drawn?")
                .expect("Failed to get user input"),
        );
        let line_order = LineOrder::choice("What order should each line be drawn in?")
            .expect("Failed to get user input");

        println!("Move your cursor to select the region where you want to draw.");
        let (start_pos, end_pos) = self.capture_screen_region();
        let scaled_img =
            ImageScaler::scale_image_to_region(&bw_img, start_pos, end_pos, scaling_mode);

        println!("Ready to draw! Press 'D' to start drawing or 'Q' to quit");
        self.wait_for_drawing_command(&scaled_img, start_pos, drawing_speed, step, line_order);
    }
}

impl Default for DrawingApp {
    fn default() -> Self {
        Self::new()
    }
}

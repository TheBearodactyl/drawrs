use crate::choices::*;
use device_query::{DeviceQuery, DeviceState};
use enigo::{Enigo, Key, KeyboardControllable, MouseButton, MouseControllable};
use image::{imageops::FilterType, DynamicImage, ImageBuffer, Luma};
use indicatif::{ProgressBar, ProgressStyle};
use native_dialog::FileDialog;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

mod choices;
mod picoseconds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    fn distance_squared(&self, other: &Point) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

struct DrawingApp {
    enigo: Enigo,
    device_state: DeviceState,
}

impl DrawingApp {
    fn new() -> Self {
        DrawingApp {
            enigo: Enigo::new(),
            device_state: DeviceState::new(),
        }
    }

    fn select_image() -> Option<String> {
        FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "gif"])
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
                self.wait_for_key(device_query::Keycode::S);

                let start = self.device_state.get_mouse().coords;
                println!("Start position captured: ({}, {})", start.0, start.1);
                println!("Move to end position and press 'E'");

                self.wait_for_key(device_query::Keycode::E);
                let end = self.device_state.get_mouse().coords;
                println!("End position captured: ({}, {})", end.0, end.1);

                (start, end)
            }
        };

        (start_pos, end_pos)
    }

    fn wait_for_key(&self, target_key: device_query::Keycode) {
        loop {
            let keys = self.device_state.get_keys();
            if keys.contains(&target_key) {
                self.wait_for_key_release(target_key);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_key_release(&self, target_key: device_query::Keycode) {
        while self.device_state.get_keys().contains(&target_key) {
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn process_image(
        image_path: &str,
    ) -> Result<ImageBuffer<Luma<u16>, Vec<u16>>, Box<dyn std::error::Error>> {
        let img = image::open(image_path)?;
        let gray_img = img.to_luma16();

        let threshold = Self::calculate_otsu_threshold(&gray_img);
        let width = gray_img.width();
        let height = gray_img.height();
        let pixels: Vec<_> = gray_img.pixels().collect();

        let binary_pixels: Vec<u16> = pixels
            .par_iter()
            .map(|pixel| if pixel[0] > threshold { 255 } else { 0 })
            .collect();

        let binary_img = ImageBuffer::from_vec(width, height, binary_pixels)
            .ok_or("Failed to create binary image")?;

        Ok(binary_img)
    }

    fn calculate_otsu_threshold(img: &ImageBuffer<Luma<u16>, Vec<u16>>) -> u16 {
        let mut histogram = [0u32; 65536];
        let total_pixels = img.width() * img.height();

        for pixel in img.pixels() {
            histogram[pixel[0] as usize] += 1;
        }

        let mut best_threshold = 0u16;
        let mut max_variance = 0.0;

        for t in 0..65536 {
            let (w0, w1, mu0, mu1) = Self::calculate_class_statistics(&histogram, t, total_pixels);

            if w0 > 0.0 && w1 > 0.0 {
                let between_class_variance = w0 * w1 * (mu0 - mu1).powi(2);
                if between_class_variance > max_variance {
                    max_variance = between_class_variance;
                    best_threshold = t as u16;
                }
            }
        }

        best_threshold
    }

    fn calculate_class_statistics(
        histogram: &[u32; 65536],
        threshold: usize,
        total: u32,
    ) -> (f64, f64, f64, f64) {
        let mut sum0 = 0u32;
        let mut sum1 = 0u32;
        let mut count0 = 0u32;
        let mut count1 = 0u32;

        for (i, &count) in histogram.iter().enumerate() {
            if i <= threshold {
                sum0 += i as u32 * count;
                count0 += count;
            } else {
                sum1 += i as u32 * count;
                count1 += count;
            }
        }

        let w0 = count0 as f64 / total as f64;
        let w1 = count1 as f64 / total as f64;
        let mu0 = if count0 > 0 {
            sum0 as f64 / count0 as f64
        } else {
            0.0
        };
        let mu1 = if count1 > 0 {
            sum1 as f64 / count1 as f64
        } else {
            0.0
        };

        (w0, w1, mu0, mu1)
    }

    fn scale_image_to_region(
        img: &ImageBuffer<Luma<u16>, Vec<u16>>,
        start_pos: (i32, i32),
        end_pos: (i32, i32),
        scaling_mode: ScalingMode,
    ) -> ImageBuffer<Luma<u16>, Vec<u16>> {
        let region_width = (end_pos.0 - start_pos.0).unsigned_abs();
        let region_height = (end_pos.1 - start_pos.1).unsigned_abs();

        let region_width = region_width.max(10);
        let region_height = region_height.max(10);

        let img_width = img.width();
        let img_height = img.height();

        println!(
            "Original image: {}x{}, Target region: {}x{}",
            img_width, img_height, region_width, region_height
        );

        match scaling_mode {
            ScalingMode::Stretch => DynamicImage::ImageLuma16(img.clone())
                .resize_exact(region_width, region_height, FilterType::Lanczos3)
                .to_luma16(),

            ScalingMode::Fit => {
                let scale_x = region_width as f64 / img_width as f64;
                let scale_y = region_height as f64 / img_height as f64;
                let scale = scale_x.min(scale_y);

                let new_width = (img_width as f64 * scale) as u32;
                let new_height = (img_height as f64 * scale) as u32;

                let scaled_img = DynamicImage::ImageLuma16(img.clone())
                    .resize(new_width, new_height, FilterType::Lanczos3)
                    .to_luma16();

                let mut canvas =
                    ImageBuffer::from_pixel(region_width, region_height, Luma([255u16]));
                let offset_x = (region_width - new_width) / 2;
                let offset_y = (region_height - new_height) / 2;

                for (x, y, pixel) in scaled_img.enumerate_pixels() {
                    if offset_x + x < region_width && offset_y + y < region_height {
                        canvas.put_pixel(offset_x + x, offset_y + y, *pixel);
                    }
                }

                canvas
            }

            ScalingMode::Fill => {
                let scale_x = region_width as f64 / img_width as f64;
                let scale_y = region_height as f64 / img_height as f64;
                let scale = scale_x.max(scale_y);

                let new_width = (img_width as f64 * scale) as u32;
                let new_height = (img_height as f64 * scale) as u32;

                let scaled_img = DynamicImage::ImageLuma16(img.clone())
                    .resize(new_width, new_height, FilterType::Lanczos3)
                    .to_luma16();

                let crop_x = if new_width > region_width {
                    (new_width - region_width) / 2
                } else {
                    0
                };
                let crop_y = if new_height > region_height {
                    (new_height - region_height) / 2
                } else {
                    0
                };

                let mut result = ImageBuffer::new(region_width, region_height);
                for (x, y, pixel) in result.enumerate_pixels_mut() {
                    let source_x = crop_x + x;
                    let source_y = crop_y + y;
                    if source_x < new_width && source_y < new_height {
                        *pixel = *scaled_img.get_pixel(source_x, source_y);
                    } else {
                        *pixel = Luma([255u16]);
                    }
                }

                result
            }

            ScalingMode::Center => {
                let mut canvas =
                    ImageBuffer::from_pixel(region_width, region_height, Luma([255u16]));
                let offset_x = if region_width > img_width {
                    (region_width - img_width) / 2
                } else {
                    0
                };
                let offset_y = if region_height > img_height {
                    (region_height - img_height) / 2
                } else {
                    0
                };

                for (x, y, pixel) in img.enumerate_pixels() {
                    let dest_x = offset_x + x;
                    let dest_y = offset_y + y;
                    if dest_x < region_width && dest_y < region_height {
                        canvas.put_pixel(dest_x, dest_y, *pixel);
                    }
                }

                canvas
            }

            ScalingMode::Tile => {
                let mut canvas = ImageBuffer::new(region_width, region_height);

                for (x, y, pixel) in canvas.enumerate_pixels_mut() {
                    let source_x = x % img_width;
                    let source_y = y % img_height;
                    *pixel = *img.get_pixel(source_x, source_y);
                }

                canvas
            }
        }
    }

    fn find_next_point_optimized(
        current: Point,
        spatial_index: &HashMap<(i32, i32), Vec<Point>>,
        visited: &HashSet<Point>,
        max_distance: i32,
    ) -> Option<Point> {
        let grid_size = max_distance.max(1);
        let current_grid_x = current.x / grid_size;
        let current_grid_y = current.y / grid_size;

        let mut best_point = None;
        let mut best_distance = i32::MAX;

        for dx in -1..=1 {
            for dy in -1..=1 {
                let grid_key = (current_grid_x + dx, current_grid_y + dy);
                if let Some(points) = spatial_index.get(&grid_key) {
                    for &point in points {
                        if !visited.contains(&point) {
                            let dist_sq = current.distance_squared(&point);
                            if dist_sq <= max_distance * max_distance && dist_sq < best_distance {
                                best_distance = dist_sq;
                                best_point = Some(point);
                            }
                        }
                    }
                }
            }
        }

        best_point
    }

    fn build_spatial_index(
        points: &HashSet<Point>,
        grid_size: i32,
    ) -> HashMap<(i32, i32), Vec<Point>> {
        let mut spatial_index: HashMap<(i32, i32), Vec<Point>> = HashMap::new();

        for &point in points {
            let grid_x = point.x / grid_size;
            let grid_y = point.y / grid_size;
            spatial_index
                .entry((grid_x, grid_y))
                .or_default()
                .push(point);
        }

        spatial_index
    }

    fn trace_line_optimized(
        start: Point,
        spatial_index: &HashMap<(i32, i32), Vec<Point>>,
        visited: &mut HashSet<Point>,
        max_distance: i32,
    ) -> Vec<Point> {
        let mut line = vec![start];
        visited.insert(start);

        while let Some(next) = Self::find_next_point_optimized(
            *line.last().unwrap(),
            spatial_index,
            visited,
            max_distance,
        ) {
            line.push(next);
            visited.insert(next);
        }

        line
    }

    fn find_connected_components_optimized(
        points: HashSet<Point>,
        max_distance: i32,
    ) -> Vec<Vec<Point>> {
        let spatial_index = Self::build_spatial_index(&points, max_distance.max(1));
        let mut visited = HashSet::new();
        let mut lines = Vec::new();

        let mut sorted_points: Vec<_> = points.into_iter().collect();
        sorted_points.sort_by_key(|p| (p.y, p.x));

        for start_point in sorted_points {
            if !visited.contains(&start_point) {
                let line = Self::trace_line_optimized(
                    start_point,
                    &spatial_index,
                    &mut visited,
                    max_distance,
                );

                if line.len() > 2 {
                    lines.push(line);
                }
            }
        }

        lines.sort_by_key(|line| std::cmp::Reverse(line.len()));
        lines
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

    fn mod_opac(&mut self, modifier: i32) {
        for _ in 0..((modifier / 10).abs()) {
            if modifier > 0 {
                &self.enigo.key_click(Key::O);
            } else {
                &self.enigo.key_click(Key::I);
            }
        }
    }

    fn calc_opac_adj(brightness: u16) -> i32 {
        if brightness <= 4096 {
            0
        } else if brightness <= 8192 {
            -1
        } else {
            1
        }
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

        let mut lines = Self::find_connected_components_optimized(black_pixels, 3);
        println!("Generated {} drawing paths", lines.len());
        let progress_style = ProgressStyle::default_bar().progress_chars("=>=");
        let pb = ProgressBar::new(lines.len() as u64);
        pb.set_style(progress_style);

        let total_lines = lines.len();
        let rng = rand::rng();

        if line_order == LineOrder::Shuffled {
            lines.shuffle(&mut rng.clone());
        }

        for line in lines.iter() {
            if line.len() < 2 {
                continue;
            }

            pb.inc(1);

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
                        let brightness = img.get_pixel(current.x as u32, current.y as u32)[0];
                        let adjustments = Self::calc_opac_adj(brightness);

                        for _ in 0..adjustments.abs() {
                            if adjustments < 0 {
                                self.enigo.key_click(Key::I);
                            } else {
                                self.enigo.key_click(Key::O);
                            }

                            thread::sleep(drawing_speed);
                        }

                        thread::sleep(drawing_speed);
                    }
                } else {
                    let abs_x = start_pos.0 + next.x;
                    let abs_y = start_pos.1 + next.y;
                    self.enigo.mouse_move_to(abs_x, abs_y);

                    if !drawing_speed.is_zero() {
                        thread::sleep(drawing_speed);
                    }

                    let brightness = img.get_pixel(next.x as u32, next.y as u32)[0];
                    let adjustments = Self::calc_opac_adj(brightness);
                    for _ in 0..adjustments.abs() {
                        if adjustments < 0 {
                            self.enigo.key_sequence("I");
                        } else {
                            self.enigo.key_sequence("O");
                        }
                        thread::sleep(Duration::from_millis(50)); // Delay to ensure Krita processes the key presses
                    }
                }
            }

            self.enigo.mouse_up(MouseButton::Left);
            thread::sleep(drawing_speed);
        }

        pb.finish_with_message(format!(
            "Drawing completed! Drew {} paths with {} total points",
            total_lines,
            lines.iter().map(|l| l.len()).sum::<usize>()
        ));
    }
}

fn main() {
    let mut app = DrawingApp::new();

    println!("Please select an image file");
    let image_path = match DrawingApp::select_image() {
        Some(path) => path,
        None => {
            println!("No image selected. Exiting...");
            return;
        }
    };

    let bw_img = match DrawingApp::process_image(&image_path) {
        Ok(img) => img,
        Err(e) => {
            println!("Error processing image: {}", e);
            return;
        }
    };

    println!("Image processed successfully!");

    let scaling_mode = app.select_scaling_mode();
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
    let (start_pos, end_pos) = app.capture_screen_region();
    let scaled_img = DrawingApp::scale_image_to_region(&bw_img, start_pos, end_pos, scaling_mode);

    println!("Ready to draw! Press 'D' to start drawing or 'Q' to quit");

    loop {
        let keys = app.device_state.get_keys();
        if keys.contains(&device_query::Keycode::D) {
            app.draw_image_optimized(&scaled_img, start_pos, drawing_speed, step, line_order);
            break;
        } else if keys.contains(&device_query::Keycode::Q) {
            println!("Quitting...");
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    std::process:exit(1);
}

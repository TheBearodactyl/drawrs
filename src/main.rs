use device_query::{DeviceQuery, DeviceState, MousePosition};
use enigo::{Enigo, MouseButton, MouseControllable};
use image::{DynamicImage, ImageBuffer, Luma};
use native_dialog::FileDialog;
use std::collections::{HashSet, VecDeque};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
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
            .add_filter("Image Files", &["png", "jpg", "jpeg"])
            .show_open_single_file()
            .ok()?
            .map(|path| path.to_string_lossy().into_owned())
    }

    fn capture_screen_region(&self) -> ((i32, i32), (i32, i32)) {
        println!("Press 'S' to start selecting region");
        loop {
            let keys = self.device_state.get_keys();
            if keys.contains(&device_query::Keycode::S) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let start_pos = self.device_state.get_mouse().coords;
        println!("Start position captured, move to end position and press 'E'");

        loop {
            let keys = self.device_state.get_keys();
            if keys.contains(&device_query::Keycode::E) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let end_pos = self.device_state.get_mouse().coords;
        (start_pos, end_pos)
    }

    fn process_image(image_path: &str) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let img = image::open(image_path).expect("Failed to open image");
        let gray_img = img.to_luma8();
        let mut binary_img = ImageBuffer::new(gray_img.width(), gray_img.height());

        for (x, y, pixel) in gray_img.enumerate_pixels() {
            binary_img.put_pixel(x, y, Luma([if pixel[0] > 128 { 255 } else { 0 }]));
        }

        binary_img
    }

    fn scale_image_to_region(
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
        start_pos: (i32, i32),
        end_pos: (i32, i32),
    ) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let width = (end_pos.0 - start_pos.0).abs() as u32;
        let height = (end_pos.1 - start_pos.1).abs() as u32;

        DynamicImage::ImageLuma8(img.clone())
            .resize(width, height, image::imageops::FilterType::Lanczos3)
            .to_luma8()
    }

    fn find_next_point(
        current: Point,
        points: &HashSet<Point>,
        max_distance: i32,
    ) -> Option<Point> {
        for dx in -max_distance..=max_distance {
            for dy in -max_distance..=max_distance {
                if dx * dx + dy * dy <= max_distance * max_distance {
                    let next = Point::new(current.x + dx, current.y + dy);
                    if points.contains(&next) {
                        return Some(next);
                    }
                }
            }
        }
        None
    }

    fn trace_line(start: Point, points: &mut HashSet<Point>, max_distance: i32) -> Vec<Point> {
        let mut line = vec![start];
        points.remove(&start);

        while let Some(next) = Self::find_next_point(*line.last().unwrap(), points, max_distance) {
            line.push(next);
            points.remove(&next);
        }

        line
    }

    fn find_connected_components(
        points: &mut HashSet<Point>,
        max_distance: i32,
    ) -> Vec<Vec<Point>> {
        let mut lines = Vec::new();

        while !points.is_empty() {
            let start = *points.iter().next().unwrap();
            let line = Self::trace_line(start, points, max_distance);

            if line.len() > 1 {
                lines.push(line);
            }
        }

        lines
    }

    fn get_black_pixels(img: &ImageBuffer<Luma<u8>, Vec<u8>>, step: u32) -> HashSet<Point> {
        let mut pixels = HashSet::new();

        for y in (0..img.height()).step_by(step as usize) {
            for x in (0..img.width()).step_by(step as usize) {
                if img.get_pixel(x, y)[0] == 0 {
                    pixels.insert(Point::new(x as i32, y as i32));
                }
            }
        }

        pixels
    }

    fn draw_image(
        &mut self,
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
        start_pos: (i32, i32),
        step: u32,
    ) {
        println!("Drawing will start in 3 seconds. Keep your cursor still!");
        thread::sleep(Duration::from_secs(3));

        let mut black_pixels = Self::get_black_pixels(img, step);
        let lines = Self::find_connected_components(&mut black_pixels, 2);

        for line in lines {
            if line.len() < 2 {
                continue;
            }

            // Move to start of line
            let abs_start_x = start_pos.0 + line[0].x;
            let abs_start_y = start_pos.1 + line[0].y;
            self.enigo
                .mouse_move_to(abs_start_x as i32, abs_start_y as i32);

            // Press mouse button
            self.enigo.mouse_down(MouseButton::Left);

            // Draw through all points
            for point in line.iter().skip(1) {
                let abs_x = start_pos.0 + point.x;
                let abs_y = start_pos.1 + point.y;
                self.enigo.mouse_move_to(abs_x as i32, abs_y as i32);
                thread::sleep(Duration::from_micros(1000));
            }

            // Release mouse button
            self.enigo.mouse_up(MouseButton::Left);
            thread::sleep(Duration::from_millis(10));
        }

        println!("Drawing completed!");
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

    let bw_img = DrawingApp::process_image(&image_path);
    println!("Move your cursor to select the region where you want to draw.");
    let (start_pos, end_pos) = app.capture_screen_region();
    let scaled_img = DrawingApp::scale_image_to_region(&bw_img, start_pos, end_pos);

    println!("Ready to draw! Press 'D' to start drawing or 'Q' to quit");
    loop {
        let keys = app.device_state.get_keys();
        if keys.contains(&device_query::Keycode::D) {
            app.draw_image(&scaled_img, start_pos, 1);
            break;
        } else if keys.contains(&device_query::Keycode::Q) {
            println!("Quitting...");
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

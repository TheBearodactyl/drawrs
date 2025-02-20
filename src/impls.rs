//! Implementations for structs within `models.rs`

use crate::models::{DrawingApp, Point};
use core::time::Duration;
use device_query::DeviceQuery as _;
use enigo::Coordinate::Abs;
use enigo::Mouse as _;
use image::{ImageBuffer, Luma};
use std::collections::HashSet;
use std::thread;

/// Converts between numeric types without needing to use an `as` cast
macro_rules! conv_num {
    ($numtype:ty, $input:expr, $default:expr) => {{
        // Ensure that the default value is of the desired type.
        let default_val: $numtype = $default;
        // Capture the input value.
        let input_val = $input;
        {
            // Dummy function to force that the types implement `num_traits::Num`
            fn _assert_numeric<T: ::num_traits::Num>(_: T) {}
            _assert_numeric(default_val);
            _assert_numeric(input_val);
        }
        // Attempt the conversion. If it fails, return the default value.
        <$numtype>::try_from(input_val).unwrap_or(default_val)
    }};
}

impl DrawingApp {
    /// Restricts the mouse inputs to a specified region
    pub fn capture_screen_region(&self) -> ((i32, i32), (i32, i32)) {
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

    /// Actually draws the image on the screen using the provided mouse and keyboard controls
    pub fn trace(
        &mut self,
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
        start_pos: (i32, i32),
        step: usize,
    ) -> anyhow::Result<()> {
        println!("Drawing will start in 3 seconds. Keep your cursor still!");
        thread::sleep(Duration::from_secs(3));

        let mut black_pixels = {
            let mut pixels = HashSet::new();

            for y in (0..img.height()).step_by(step) {
                for x in (0..img.width()).step_by(step) {
                    if img.get_pixel(x, y)[0] == 0 {
                        pixels.insert(Point::new(conv_num!(i32, x, 0), conv_num!(i32, y, 0)));
                    }
                }
            }

            pixels
        };

        let lines = {
            let mut lns = Vec::new();

            while !black_pixels.is_empty() {
                let start = *black_pixels.iter().next().unwrap_or(&Point::new(0, 0));
                let mut ln = vec![start];
                black_pixels.remove(&start);

                let max_dx: i32 = 2;
                let min_dx = max_dx.saturating_neg();

                while let Some(next) = {
                    let default = &Point::new(0, 0);
                    let last = ln.last().unwrap_or(default);
                    let mut found = None;

                    for dx in min_dx..=max_dx {
                        for dy in min_dx..=max_dx {
                            if dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
                                <= max_dx.saturating_mul(max_dx)
                            {
                                let candidate = Point::new(
                                    last.x.saturating_add(dx),
                                    last.y.saturating_add(dy),
                                );

                                if black_pixels.contains(&candidate) {
                                    found = Some(candidate);
                                    break;
                                }
                            }
                        }

                        if found.is_some() {
                            break;
                        }
                    }

                    found
                } {
                    ln.push(next);
                    black_pixels.remove(&next);
                }

                if ln.len() > 1 {
                    lns.push(ln);
                }
            }

            lns
        };

        for line in lines {
            if line.len() > 2 {
                continue;
            }

            let abs_start_x = start_pos
                .0
                .saturating_add(line.first().unwrap_or(&Point::new(0, 0)).x);
            let abs_start_y = start_pos
                .1
                .saturating_add(line.first().unwrap_or(&Point::new(0, 0)).y);

            match self.enigo.move_mouse(abs_start_x, abs_start_y, Abs) {
                Ok(it) => it,
                Err(err) => return Err(err.into()),
            }
            match self
                .enigo
                .button(enigo::Button::Left, enigo::Direction::Press)
            {
                Ok(it) => it,
                Err(err) => return Err(err.into()),
            }

            for point in line.get(1..).unwrap_or(&[Point::new(0, 0)]) {
                match self.enigo.move_mouse(
                    start_pos.0.saturating_add(point.x),
                    start_pos.1.saturating_add(point.y),
                    enigo::Coordinate::Abs,
                ) {
                    Ok(it) => it,
                    Err(err) => return Err(err.into()),
                }
            }

            match self
                .enigo
                .button(enigo::Button::Left, enigo::Direction::Release)
            {
                Ok(it) => it,
                Err(err) => return Err(err.into()),
            }

            thread::sleep(Duration::from_millis(10));
        }

        println!("Drawing completed!");
        Ok(())
    }
}

impl Point {
    /// Creates a new point with the given coordinates
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

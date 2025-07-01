use image::{ImageBuffer, Luma};
use rayon::prelude::*;
use std::cmp::min;

use crate::choices::ImageProcessingMethod;

fn calculate_window_size(img: &ImageBuffer<Luma<u16>, Vec<u16>>) -> u32 {
    ((min(img.dimensions().0, img.dimensions().1) as f32 * 0.05).round() as u32).clamp(5, 50)
}

/// Processor for image binarization operations
///
/// Provides functionaility to convert images to binary format using
/// Otsu's thresholding method with parallel processing.
pub struct ImageProcessor;

impl ImageProcessor {
    /// Loads an image, converts it to grayscale, and binarizes it using Otsu's method
    ///
    /// # Arguments
    /// * `image_path` - Path to the input image file
    ///
    /// # Returns
    /// - `Ok(ImageBuffer)`: Binary image buffer with white pixels (255) where original > threshold, black(0) otherwise
    /// - `Err`: If image loading fails or buffer creation fails
    ///
    /// # Process
    /// 1. Load image from path
    /// 2. Convert to 16-bit grayscale
    /// 3. Calculate Otsu threshold
    /// 4. Apply threshold in parallel to create binary image
    /// 5. Return new image buffer
    ///
    /// # Example
    /// ```rs
    /// let binary_img = ImageProcessor::process_image("input.png")?;
    /// binary_img.save("binary.png")?;
    /// ```
    pub fn process_image(
        image_path: &str,
        processing_method: ImageProcessingMethod,
    ) -> Result<ImageBuffer<Luma<u16>, Vec<u16>>, Box<dyn std::error::Error>> {
        let img = image::open(image_path)?;
        let gray_img = img.to_luma16();

        let threshold = match processing_method {
            ImageProcessingMethod::Otsu => Self::calculate_otsu_threshold(&gray_img),
            ImageProcessingMethod::Kapur => Self::calculate_kapur_threshold(&gray_img),
            ImageProcessingMethod::Wolfs => Self::calculate_wolf_threshold(&gray_img),
            ImageProcessingMethod::Bernsens => Self::calculate_bernsen_threshold(&gray_img),
            ImageProcessingMethod::Sauvola => Self::calculate_sauvola_threshold(&gray_img),
        };
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

    /// Calculates optimal threshold for binarization using Otsu's method
    ///
    /// # Arguments
    /// * `img` - 16-bit grayscale image reference
    ///
    /// # Returns
    /// Optimal threshold value (0-65535) that maximizes inter-class variance
    ///
    /// # Process
    /// 1. Build 16-bit histogram (0-65535)
    /// 2. Iterate all possible thresholds:
    ///     - Split histogram into foreground/background
    ///     - Calculate class weights and means
    ///     - Compute between-class variance
    /// 3. Return threshold with maximum variance
    pub fn calculate_otsu_threshold(img: &ImageBuffer<Luma<u16>, Vec<u16>>) -> u16 {
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

    pub fn calculate_kapur_threshold(img: &ImageBuffer<Luma<u16>, Vec<u16>>) -> u16 {
        let (width, height) = img.dimensions();
        let mut histogram = [0u32; 65536];

        for pixel in img.pixels() {
            histogram[pixel.0[0] as usize] += 1;
        }

        let total_pixels = (width * height) as f64;
        let mut max_entropy = f64::MIN;
        let mut best_threshold = 0u16;

        for threshold in 0..65536 {
            let mut w0 = 0.0;
            let mut w1 = 0.0;
            let mut sum0 = 0.0;
            let mut sum1 = 0.0;

            for (i, &count) in histogram.iter().enumerate() {
                let p = count as f64 / total_pixels;

                if i <= threshold as usize {
                    w0 += p;
                    if p > 0.0 {
                        sum0 += p * (p / w0).ln();
                    }
                } else {
                    w1 += p;
                    if p > 0.0 {
                        sum1 += p * (p / w1).ln();
                    }
                }
            }

            let entropy = -sum0 - sum1;
            if entropy > max_entropy {
                max_entropy = entropy;
                best_threshold = threshold as u16;
            }
        }

        best_threshold
    }

    pub fn calculate_sauvola_threshold(img: &ImageBuffer<Luma<u16>, Vec<u16>>) -> u16 {
        let (width, height) = img.dimensions();
        let mut integral = vec![0u64; (width * height) as usize];
        let mut integral_sq = vec![0u64; (width * height) as usize];

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let pixel = img.get_pixel(x, y)[0] as u64;
                integral[idx] = pixel
                    + if x > 0 { integral[idx - 1] } else { 0 }
                    + if y > 0 {
                        integral[idx - width as usize]
                    } else {
                        0
                    }
                    - if x > 0 && y > 0 {
                        integral[idx - width as usize - 1]
                    } else {
                        0
                    };
                integral_sq[idx] = pixel * pixel
                    + if x > 0 { integral_sq[idx - 1] } else { 0 }
                    + if y > 0 {
                        integral_sq[idx - width as usize]
                    } else {
                        0
                    }
                    - if x > 0 && y > 0 {
                        integral_sq[idx - width as usize - 1]
                    } else {
                        0
                    };
            }
        }

        let mut threshold_values = Vec::new();
        let window_size = calculate_window_size(img);
        let k = 0.5;
        let r = 128.0;

        for y in window_size..height - window_size {
            for x in window_size..width - window_size {
                let x0 = x - window_size;
                let y0 = y - window_size;
                let x1 = x + window_size;
                let y1 = y + window_size;

                let area = ((x1 - x0) * (y1 - y0)) as f64;
                let sum = integral[(y1 * width + x1) as usize]
                    - integral[(y0 * width + x1) as usize]
                    - integral[(y1 * width + x0) as usize]
                    + integral[(y0 * width + x0) as usize];
                let sum_sq = integral_sq[(y1 * width + x1) as usize]
                    - integral_sq[(y0 * width + x1) as usize]
                    - integral_sq[(y1 * width + x0) as usize]
                    + integral_sq[(y0 * width + x0) as usize];

                let mean = sum as f64 / area;
                let variance = (sum_sq as f64 / area) - (mean * mean);
                let std_dev = variance.sqrt();

                let threshold = mean * (1.0 + k * (std_dev / r - 1.0));
                threshold_values.push(threshold as u16);
            }
        }

        threshold_values.sort();
        threshold_values[threshold_values.len() / 2]
    }

    pub fn calculate_bernsen_threshold(img: &ImageBuffer<Luma<u16>, Vec<u16>>) -> u16 {
        let (width, height) = img.dimensions();
        let window_size = 15;
        let mut threshold_values = Vec::new();

        for y in window_size..height - window_size {
            for x in window_size..width - window_size {
                let mut min_val = u16::MAX;
                let mut max_val = u16::MIN;

                for dy in -(window_size as i32)..=(window_size as i32) {
                    for dx in -(window_size as i32)..=(window_size as i32) {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                            let pixel = img.get_pixel(nx as u32, ny as u32)[0];
                            min_val = min_val.min(pixel);
                            max_val = max_val.max(pixel);
                        }
                    }
                }

                let threshold = ((min_val as f64 + max_val as f64) / 2.0) as u16;
                threshold_values.push(threshold);
            }
        }

        threshold_values.sort();
        threshold_values[threshold_values.len() / 2]
    }

    pub fn calculate_wolf_threshold(img: &ImageBuffer<Luma<u16>, Vec<u16>>) -> u16 {
        let (width, height) = img.dimensions();
        let mut min_img = ImageBuffer::new(width, height);
        let mut max_img = ImageBuffer::new(width, height);
        let window_size = calculate_window_size(img) as i32;
        let k = 0.5;

        for y in 0..height {
            for x in 0..width {
                let mut min_val = u16::MAX;
                let mut max_val = u16::MIN;
                for dy in -window_size..=window_size {
                    for dx in -window_size..=window_size {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                            let pixel = img.get_pixel(nx as u32, ny as u32)[0];
                            min_val = min_val.min(pixel);
                            max_val = max_val.max(pixel);
                        }
                    }
                }
                min_img.put_pixel(x, y, Luma([min_val]));
                max_img.put_pixel(x, y, Luma([max_val]));
            }
        }

        let mut contrast_sum = 0.0;
        let mut weight_sum = 0.0;

        for pixel in img.pixels() {
            let min_pixel = min_img.get_pixel(pixel[0] as u32, pixel[1] as u32)[0] as f64;
            let max_pixel = max_img.get_pixel(pixel[0] as u32, pixel[1] as u32)[0] as f64;
            let contrast = max_pixel - min_pixel;
            contrast_sum += contrast * (pixel[0] as f64);
            weight_sum += contrast;
        }

        if weight_sum > 0.0 {
            (contrast_sum / weight_sum * k) as u16
        } else {
            32768
        }
    }

    /// Calculates class statistics for given threshold
    ///
    /// # Arguments
    /// * `histogram` - Pixel intensity distribution
    /// * `threshold` - Current threshold to evaluate
    /// * `total` - Total number of pixels in image
    ///
    /// # Returns
    /// Tuple containing:
    /// - `w0`: Background weight (fraction of pixels below threshold)
    /// - `w1`: Foreground weight (fraction of pixels above threshold)
    /// - `mu0`: Background mean intensity
    /// - `mu1`: Foreground mean intensity
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
}

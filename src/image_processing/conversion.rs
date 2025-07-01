use image::{ImageBuffer, Luma};
use rayon::prelude::*;

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

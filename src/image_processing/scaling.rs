use crate::choices::ScalingMode;
use image::{imageops::FilterType, DynamicImage, ImageBuffer, Luma};

/// Provides image scaling operations with various resizing methods
///
/// Supports multiple scaling modes to fit images into target regions while preserving
/// aspect ratio or filling space as needed. All operations work on 16-bit grayscale images.
pub struct ImageScaler;

impl ImageScaler {
    /// Scales an image to fit within a specified region using the selected scaling mode
    ///
    /// # Arguments
    /// * `img` - Input 16-bit grayscale image to scale
    /// * `start_pos` - (x, y) coordinates of region start point (top-left corner)
    /// * `end_pos` - (x, y) coordinates of region end point (bottom-right corner)
    /// * `scaling_mode` - [`ScalingMode`] strategy to use for resizing
    ///
    /// # Returns
    /// New image buffer sized to the region dimensions (width = |end_x - start_x|, height = |end_y - start_y|)
    ///
    /// # Note
    /// Region dimensions are enforced to be at least 10x10 pixels. Coordinates can be in any order
    /// (top-left to bottom-right or bottom-right to top-left) since absolute differences are used.
    ///
    /// # Scaling Modes
    /// - **Stretch**: Ignores aspect ratio, stretches to exact region size
    /// - **Fit**: Maintains aspect ratio, fits entirely within region with letterboxing
    /// - **Fill**: Maintains aspect ratio, fills entire region with cropping
    /// - **Center**: No scaling, centers original image with padding
    /// - **Tile**: Repeats image in both directions like a mosaic
    ///
    /// # Example
    /// ```ignore
    /// let scaled = ImageScaler::scale_image_to_region(
    ///     &input_img,
    ///     (100, 100),
    ///     (300, 200),
    ///     ScalingMode::Fit
    /// );
    /// ```
    pub fn scale_image_to_region(
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
}

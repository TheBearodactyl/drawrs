use crate::utils::duration::DurExt;
use inquiry::Choice;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Choice, PartialEq)]
pub enum ImageProcessingMethod {
    /// Otsu's Method - Best for most use-cases (default)
    Otsu,

    /// Kapur's Entropy - Best for textured/heterogeneous images
    Kapur,

    /// Wolf's Method - Best for degraded images
    Wolfs,

    /// Bernsen's Method - Best for low-contrast images
    Bernsens,

    /// Sauvola's Method - Best for images with noisy/textured backgrounds
    Sauvola,
}

#[derive(Debug, Clone, Copy, Choice, PartialEq)]
pub enum ScalingMode {
    /// Stretch - Fills entire region (may distort)
    Stretch,

    /// Fit - Scales to fit within region (maintains aspect ratio)
    Fit,

    /// Fill - Scales to fill region completely (may crop edges)
    Fill,

    /// Center - Original size, centered in region
    Center,

    /// Tile - Repeats image to fill region
    Tile,
}

#[derive(Debug, Clone, Copy, Choice, PartialEq)]
pub enum RegionPickMode {
    /// Interactive - Interactively choose 2 coordinates to select the region
    Interactive,

    /// Manual - Input 2 coordinates to select the region
    Manual,
}

#[derive(Debug, Clone, Copy, Choice, PartialEq)]
pub enum DrawingAccuracy {
    /// Fast - Makes the drawing go faster at the cost of accuracy
    Fast,

    /// Balanced - Balances speed and accuracy
    Balanced,

    /// Accurate - Makes the drawing more accurate at the cost of speed
    Accurate,
}

#[derive(Debug, Clone, Copy, Choice, PartialEq)]
pub enum DrawingSpeed {
    /// Universe Annihilating (1ps/line) (BREAKS SOME APPS)
    UniverseAnnihilating,

    /// Ultra-Fast (1ms/line)
    UltraFast,

    /// Fast (10ms/line)
    Fast,

    /// Medium (50ms/line)
    Medium,

    /// Slow (200ms/line)
    Slow,
}

#[derive(Debug, Clone, Copy, Choice, PartialEq)]
pub enum LineOrder {
    /// In Order - Draw each line in order
    InOrder,

    /// Shuffled - Shuffle the order of each drawn line before starting
    Shuffled,
}

pub fn get_step(drawing_accuracy: DrawingAccuracy) -> i32 {
    match drawing_accuracy {
        DrawingAccuracy::Fast => 3,
        DrawingAccuracy::Balanced => 2,
        DrawingAccuracy::Accurate => 1,
    }
}

pub fn get_speed(speed: DrawingSpeed) -> Duration {
    match speed {
        DrawingSpeed::UniverseAnnihilating => {
            let confirmation = inquire::prompt_confirmation(
                "This will break things. Are you sure you want to use this speed?",
            )
            .expect("Failed to read confirmation");

            if confirmation {
                Duration::from_picos(1)
            } else {
                panic!("Cancelled operation");
            }
        }
        DrawingSpeed::UltraFast => Duration::from_micros(1),
        DrawingSpeed::Fast => Duration::from_micros(10),
        DrawingSpeed::Medium => Duration::from_micros(50),
        DrawingSpeed::Slow => Duration::from_micros(200),
    }
}

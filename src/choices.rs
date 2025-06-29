use crate::picoseconds::DurExt;
use inquiry::Choice;
use std::time::Duration;

pub(crate) trait Description {
    fn description(&self) -> &'static str;
}

#[macro_export]
macro_rules! choice {
    ($enum_name:ident, $($variant:ident => $desc:expr),+) => {
	    #[derive(Debug, Copy, Clone, Choice)]
	    pub(crate) enum $enum_name {
		    $($variant,)+
	    }

	    impl Description for $enum_name {
		    fn description(&self) -> &'static str {
			    match self {
				    $($enum_name::$variant => $desc),+
			    }
		    }
	    }

	    impl std::fmt::Display for $enum_name {
		    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			    write!(f, "{}", self.description())
		    }
	    }
    };
}

choice!(ScalingMode,
    Stretch => "Stretch - Fills entire region (may distort)",
    Fit => "Fit - Scales to fit within region (maintains aspect ratio)",
    Fill => "Fill - Scales to fill region completely (may crop edges)",
    Center => "Center - Original size, centered in region",
    Tile => "Tile - Repeats image to fill region"
);

choice!(RegionPickMode,
    Interactive => "Interactive - Interactively choose 2 coordinates to select the region",
    Manual => "Manual - Input 2 coordinates to select the region"
);

choice!(DrawingAccuracy,
    Fast => "Fast - Makes the drawing go faster at the cost of accuracy",
    Balanced => "Balanced - Balances speed and accuracy",
    Accurate => "Accurate - Makes the drawing more accurate at the cost of speed"
);

choice!(DrawingSpeed,
    UniverseAnnihilating => "Universe Annihilating (1ps/line) (BREAKS SOME APPS)",
    UltraFast => "Ultra-Fast (1ms/line)",
    Fast => "Fast (10ms/line)",
    Medium => "Medium (50ms/line)",
    Slow => "Slow (200ms/line)"
);

pub(crate) fn get_step(drawing_accuracy: DrawingAccuracy) -> i32 {
    match drawing_accuracy {
        DrawingAccuracy::Fast => 3,
        DrawingAccuracy::Balanced => 2,
        DrawingAccuracy::Accurate => 1,
    }
}

pub(crate) fn get_speed(speed: DrawingSpeed) -> Duration {
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

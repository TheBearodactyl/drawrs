pub use self::drawing::*;

pub(crate) mod drawing;

/// Trait for types that provide a human-readable description
///
/// Used to display enum variants in user interfaces with friendly text
/// rather than the base identifier names for each variant.
pub trait Description {
    /// Returns a static string description of the implementing type
    fn description(&self) -> &'static str;
}

/// Creates enums with a method to ask for a user to choose a variant on the CLI
///
/// This macro generates:
/// 1. An enum with specified variants
/// 2. Standard trait implementations (Debug, Copy, Clone, etc.)
/// 3. Ord/PartialOrd/Eq/PartialEq for ordering/comparison
/// 4. [`Description`] trait implementation
/// 5. Display trait implementation
///
/// # Syntax
/// `choice!(EnumName, Variant => "Description", ...)`
///
/// # Example
/// ```
/// choice!(ColorChoice,
///     Red => "Vibrant Red",
///     Green => "Forest Green",
///     Blue => "Deep Ocean Blue"
/// );
///
/// // Creates enum equivalent to:
/// // #[derive(Debug, Copy, Clone, Choice, Ord, PartialOrd, Eq, PartialEq)]
/// // pub enum ColorChoice { Red, Green, Blue }
/// //
/// // impl Description for ColorChoice {
/// //     fn description(&self) -> &'static str {
/// //         match self {
/// //             ColorChoice::Red => "Vibrant Red",
/// //             ColorChoice::Green => "Forest Green",
/// //             ColorChoice::Blue => "Deep Ocean Blue",
/// //         }
/// //     }
/// // }
/// //
/// // impl std::fmt::Display for ColorChoice {
/// //     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
/// //         write!(f, "{}", self.description())
/// //     }
/// // }
/// ```
///
/// # Usage in CLI prompts
/// The generated enums work particularly well with the `inquiry` crate
/// for interactive command-line selection interfaces:
/// ```
/// use inquiry::*;
///
/// let selection = ColorChoice::choice("Please select a color")
///     .expect("Failed to get user input");
/// ```
#[macro_export]
macro_rules! choice {
    ($enum_name:ident, $($variant:ident => $desc:expr),+) => {
        #[derive(Debug, Copy, Clone, inquiry::Choice, Ord, PartialOrd, Eq, PartialEq)]
        pub enum $enum_name {
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

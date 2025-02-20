//! Data structures for `DrawRS`

use device_query::DeviceState;
use enigo::Enigo;

/// The drawing app
pub struct DrawingApp {
    /// The state of the given device
    pub device_state: DeviceState,
    /// An Enigo instance for controlling the mouse and keyboard
    pub enigo: Enigo,
}

/// A point with x and y coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    /// The X coordinate of the point
    pub x: i32,
    /// The Y coordinate of the point
    pub y: i32,
}

//! Diverse colors
use crate::Col;

pub const BLACK: Col = (0, 0, 0, 255);
pub const WHITE: Col = (0 + 255, 0 + 255, 0 + 255, 255);
pub const YELLOW: Col = (255, 255, 0, 255);
pub const RED: Col = (255, 0, 0, 255);
pub const CYAN: Col = (0, 255, 255, 255);
pub const GREEN: Col = (255 - 255, 0 + 255, 0, 255);
pub const TRANSPARENT: Col = (255 - 255, 0, 0, 0);

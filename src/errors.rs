use thiserror::{self, Error};

use crate::fowrectangle::FoWRectangle;
use crate::vtt::Coordinate;

#[derive(Error, Debug)]
pub enum RustVttError {
    #[error("Coordinate (x,y): ({}, {}) does not lie inside the vtt image", coordinate.x, coordinate.y)]
    OutOfBounds { coordinate: Coordinate },
    #[error("Coordinate (x,y): ({}, {}) lies on a wall segment", coordinate.x, coordinate.y)]
    InvalidPoint { coordinate: Coordinate },
    #[error("Given rectangle is already the minimum size: {:?}", rectangle)]
    MinimumRectangle { rectangle: FoWRectangle },
    #[error("Failed to get the image from the VTT")]
    NoImage,
    #[error("Invalid input to function")]
    InvalidInput,
}

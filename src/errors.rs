use thiserror::{self, Error};

use crate::vtt::Coordinate;

#[derive(Error, Debug)]
pub enum RustVttError {
    #[error("Coordinate (x,y): ({}, {}) does not lie inside the vtt image", coordinate.x, coordinate.y)]
    OutOfBounds { coordinate: Coordinate },
    #[error("Coordinate (x,y): ({}, {}) lies on a wall segment", coordinate.x, coordinate.y)]
    InvalidPoint { coordinate: Coordinate },
}

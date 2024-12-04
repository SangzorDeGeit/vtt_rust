use thiserror::{self, Error};

use crate::vtt::Coordinate;

#[derive(Error, Debug)]
pub enum RustVttError {
    #[error("Coordinate (x,y): ({}, {}) does not lie inside the vtt image", coordinate.x, coordinate.y)]
    OutOfBounds { coordinate: Coordinate },
}

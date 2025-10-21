// test comment for test 3
use crate::{
    errors::RustVttError,
    fog_of_war::{self, FogOfWar, Operation},
    helper::{calculate_direct_los, get_line_segments},
};
use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use geo::Coord;
use serde::{Deserialize, Serialize};
use std::{f64, fs::File, io::Write, path::Path};

/// The main VTT structure containing all the data that is in the .vtt file.
#[derive(Serialize, Deserialize)]
pub struct VTT {
    format: f32,
    resolution: Resolution,
    line_of_sight: Vec<Vec<Coordinate>>,
    objects_line_of_sight: Vec<Vec<Coordinate>>,
    portals: Vec<Portal>,
    environment: Environment,
    lights: Vec<Light>,
    #[serde(skip)]
    fog_of_war: Option<FogOfWar>,
    image: String,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct Resolution {
    pub map_origin: Coordinate,
    pub map_size: Coordinate,
    pub pixels_per_grid: i32,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct Light {
    position: Coordinate,
    range: f64,
    intensity: f64,
    color: String,
    shadows: bool,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct Environment {
    baked_lighting: bool,
    ambient_light: Option<String>,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct Portal {
    position: Coordinate,
    bounds: Vec<Coordinate>,
    rotation: f64,
    closed: bool,
    freestanding: bool,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Coordinate {
    pub x: f64,
    pub y: f64,
}

/// A 2d coordinate represented by pixels
#[derive(Clone)]
pub struct PixelCoordinate {
    pub x: i32,
    pub y: i32,
}

impl PixelCoordinate {
    /// Create a PixelCoordinate from a coordinate represented by floats and a resolution
    pub fn from(coordinate: &Coordinate, pixels_per_grid: i32) -> Self {
        let x = coordinate.x * pixels_per_grid as f64;
        let y = coordinate.y * pixels_per_grid as f64;
        Self {
            x: x as i32,
            y: y as i32,
        }
    }
}

impl Into<Coord> for Coordinate {
    fn into(self) -> Coord {
        Coord {
            x: self.x,
            y: self.y,
        }
    }
}

impl VTT {
    /// Return the origin point of the VTT in squares
    pub fn origin(&self) -> &Coordinate {
        return &self.resolution.map_origin;
    }

    /// Return the size of the VTT in squares
    pub fn size(&self) -> &Coordinate {
        return &self.resolution.map_size;
    }

    /// Returns the pixels per square for the VTT.
    ///
    /// # Example
    /// a returned value of 256 means that one grid square is 256x256 pixels
    pub fn pixels_per_grid(&self) -> i32 {
        return self.resolution.pixels_per_grid;
    }

    /// Add fog of war to cover the entire image
    pub fn fow_hide_all(&mut self) {
        self.fog_of_war = Some(FogOfWar::new(&self.resolution));
    }

    /// Remove fog of war from the entire image
    pub fn fow_show_all(&mut self) {
        self.fog_of_war = None;
    }

    /// Given a coordinate on the image, this function should show or hide everything that a person
    /// standing at this coordinate could see, any objects blocking line of sight (defined in the
    /// objects_line_of_sight parameter) are disregarded.
    /// ## `pov`
    /// The coordinate at which the person you want to reveal area for is standing
    /// ## `around_walls`
    /// Whether the person at the pov point can look around walls perfectly. When false, this will
    /// function as a 'line of sight' fog of war update.
    pub fn fow_change(
        &mut self,
        pov: Coordinate,
        around_walls: bool,
        operation: Operation,
    ) -> Result<(), RustVttError> {
        // First check if the given coordinate is not on or out of the bounds of the grid
        if pov.x <= self.origin().x || self.size().x <= pov.x {
            return Err(RustVttError::OutOfBounds { coordinate: pov });
        }
        if pov.y <= self.origin().y || self.size().y <= pov.y {
            return Err(RustVttError::OutOfBounds { coordinate: pov });
        }
        // Check if the coordinate is not on a wall line
        let wall_segments = get_line_segments(&self.line_of_sight);
        for wall in &wall_segments {
            // to find if point (x,y) is on the slope of line with start (x1, y1) and end (x2, y2)
            // use the following equation:
            // (y-y1)*(x2-x1)=(y2-y1)*(x-x1)
            let x1 = wall.start_point().x();
            let y1 = wall.start_point().y();
            let x2 = wall.end_point().x();
            let y2 = wall.end_point().y();
            if (pov.y - y1) * (x2 - x1) != (y2 - y1) * (pov.x - x1) {
                continue;
            }
            if pov.x <= x1.min(x2) || x1.max(x1) <= pov.x {
                continue;
            }
            if pov.y <= y1.min(y2) || y1.max(y1) <= pov.y {
                continue;
            }
            return Err(RustVttError::InvalidPoint { coordinate: pov });
        }

        if around_walls {
            todo!("Implement calculation for around walls line of sight")
        } else {
            let line_of_sight_polygon =
                calculate_direct_los(pov, &wall_segments, self.origin(), self.size());
        }

        match operation {
            Operation::HIDE => todo!(),
            Operation::SHOW => todo!(),
        }
    }

    /// Save the base64 encoded image of this vtt to a .png file.
    /// ## `path`
    /// The path to the file that the image will be exported to **excluding** the extension.
    /// # Example
    /// `save_image("path/to/filename")`
    pub fn save_img_raw<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let decoded = BASE64_STANDARD.decode(self.image.as_str())?;
        let mut file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)?;
        file.write_all(&decoded)?;
        Ok(())
    }

    /// Apply all vtt data (fog of war, lighting, etc.) to the image stored in this vtt and save it to a .png file. This
    /// function will **not** overwrite the existing image stored in the vtt.  
    pub fn save_img<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // clone the image
        // self.fog_of_war.apply_to_image(image);
        // self.environment.apply_to_image(image);
        // self.lights.apply_to_image(image);
        // save the image
        todo!("Implement this function")
    }
}

#[cfg(test)]
mod tests {
    use crate::open_vtt;

    #[test]
    fn vtt_origin() {
        let vtt = open_vtt("tests/resources/example1.dd2vtt")
            .expect("Could not open file example1.dd2vtt");
        let origin = vtt.origin();
        assert_eq!(
            origin.x, 0.0,
            "x origin did not match. Expected 0.0, found {}",
            origin.x
        );
        assert_eq!(
            origin.y, 0.0,
            "y origin did not match. Expected 0.0, found {}",
            origin.y
        );
    }

    #[test]
    fn vtt_size() {
        let vtt = open_vtt("tests/resources/example1.dd2vtt")
            .expect("Could not open file example1.dd2vtt");
        let size = vtt.size();
        assert_eq!(
            size.x, 27.0,
            "x size did not match. Expected 27.0, found {}",
            size.x
        );
        assert_eq!(
            size.y, 15.0,
            "y size did not match. Expected 15.0, found {}",
            size.y
        );
    }

    #[test]
    fn vtt_pixels_per_grid() {
        let vtt = open_vtt("tests/resources/example1.dd2vtt")
            .expect("Could not open file example1.dd2vtt");
        assert_eq!(
            vtt.pixels_per_grid(),
            256,
            "pixels per grid did not match. Expected 256, found {}",
            vtt.pixels_per_grid()
        );
    }

    #[test]
    fn vtt_save_img() {
        let vtt = open_vtt("tests/resources/The Pig and Whistle tavern.uvtt")
            .expect("Could not open file the pig and whistle tavern.uvtt");
        vtt.save_img_raw("tests/resources/tavern.png")
            .expect("Failed to save to png");
    }
}

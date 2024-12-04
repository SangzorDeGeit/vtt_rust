// test comment for test 3
use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use geo::Coord;
use std::{f64, fs::File, io::Write, path::Path};

use crate::{errors::RustVttError, fog_of_war::FogOfWar};
use serde::{Deserialize, Serialize};

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
    fog_of_war: FogOfWar,
    image: String,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct Resolution {
    map_origin: Coordinate,
    map_size: Coordinate,
    pixels_per_grid: i32,
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
    pub fn fow_hide_all(&mut self) -> &mut Self {
        self.fog_of_war.hide_all();
        return self;
    }

    /// Remove fog of war from the entire image
    pub fn fow_show_all(&mut self) -> &mut Self {
        self.fog_of_war.show_all();
        return self;
    }

    /// Given a coordinate on the image, this function should show everything that a person
    /// standing at this coordinate could see, any objects blocking line of sight (defined in the
    /// objects_line_of_sight parameter) are disregarded.
    /// ## `pov`
    /// The coordinate at which the person you want to reveal area for is standing
    /// ## `around_walls`
    /// Whether the person at the pov point can look around walls perfectly. When false, this will
    /// function as a 'line of sight' fog of war reveal.
    pub fn fow_show(&mut self, pov: Coordinate, around_walls: bool) -> Result<(), RustVttError> {
        // this implementation will be around walls false for now
        // First check if the given coordinate is not on the bounds of the grid
        if pov.x >= self.size().x || pov.x < self.origin().x {
            return Err(RustVttError::OutOfBounds { coordinate: pov });
        }
        if pov.y >= self.size().y || pov.y < self.origin().y {
            return Err(RustVttError::OutOfBounds { coordinate: pov });
        }
        // Then check if the coordinate is not on a wall line

        Ok(())
    }

    /// Given a coordinate on the image, this function should hide everything that a person
    /// standing at this coordinate could see. See [`fow_show`][crate::vtt::VTT::fow_show()] for param specifications.
    pub fn fow_hide(&mut self, pov: Coordinate, around_walls: bool) {
        todo!("Implement this function");
    }

    /// Save the base64 encoded image of this vtt to a .png file.
    /// ## `path`
    /// The path to the file that the image will be exported to **excluding** the extension.
    /// # Example
    /// `save_image("path/to/filename")`
    pub fn save_img_raw<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // you can do path.as_ref() to get the path object
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

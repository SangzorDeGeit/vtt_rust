use crate::{
    errors::RustVttError,
    fog_of_war::{FogOfWar, Operation},
    helper::{calculate_direct_los, calculate_indirect_los, get_line_segments},
};
use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use geo::{Coord, LineString, Polygon, Scale};
use image::{
    save_buffer, DynamicImage, ExtendedColorType, GenericImageView, ImageBuffer, ImageReader, Rgba,
    RgbaImage,
};
use serde::{Deserialize, Serialize};
use std::{
    f64,
    fs::File,
    io::{Cursor, Write},
    path::Path,
};

/// A VTT struct containing all data that is in the .vtt file without fog of war.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VTTPartial {
    format: f32,
    resolution: Resolution,
    line_of_sight: Vec<Vec<Coordinate>>,
    objects_line_of_sight: Vec<Vec<Coordinate>>,
    portals: Vec<Portal>,
    environment: Environment,
    lights: Vec<Light>,
    image: String,
}

/// The main VTT structure containing all the data that is in the .vtt file.
#[derive(Debug, Clone)]
pub struct VTT {
    format: f32,
    resolution: Resolution,
    line_of_sight: Vec<Vec<Coordinate>>,
    objects_line_of_sight: Vec<Vec<Coordinate>>,
    portals: Vec<Portal>,
    environment: Environment,
    lights: Vec<Light>,
    fog_of_war: FogOfWar,
    image: String,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Resolution {
    pub map_origin: Coordinate,
    pub map_size: Coordinate,
    pub pixels_per_grid: i32,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Light {
    position: Coordinate,
    range: f64,
    intensity: f64,
    color: String,
    shadows: bool,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Environment {
    baked_lighting: bool,
    ambient_light: Option<String>,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Portal {
    position: Coordinate,
    bounds: Vec<Coordinate>,
    rotation: f64,
    closed: bool,
    freestanding: bool,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Coordinate {
    pub x: f64,
    pub y: f64,
}

/// A 2d coordinate represented by pixels
#[derive(Clone, Debug, Copy, PartialEq)]
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

    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Into<Coord> for PixelCoordinate {
    fn into(self) -> Coord {
        Coord {
            x: self.x as f64,
            y: self.y as f64,
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

impl VTTPartial {
    /// Convert the partial vtt to a vtt struct that contains fog of war
    pub fn to_vtt(self) -> VTT {
        let fog_of_war = FogOfWar::new(self.resolution.clone());
        VTT {
            format: self.format,
            resolution: self.resolution,
            line_of_sight: self.line_of_sight,
            objects_line_of_sight: self.objects_line_of_sight,
            portals: self.portals,
            environment: self.environment,
            lights: self.lights,
            fog_of_war,
            image: self.image,
        }
    }
}

impl VTT {
    /// Return the format of the VTT
    pub fn format(&self) -> f32 {
        self.format
    }

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
        self.fog_of_war.hide_all();
    }

    /// Remove fog of war from the entire image
    pub fn fow_show_all(&mut self) {
        self.fog_of_war.show_all();
    }

    /// Get the fog of war of the vtt
    pub fn get_fow(&self) -> &FogOfWar {
        &self.fog_of_war
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

        let mut line_of_sight_polygon: Polygon;
        if around_walls {
            line_of_sight_polygon = calculate_indirect_los(pov, &wall_segments);
        } else {
            line_of_sight_polygon =
                calculate_direct_los(pov, &wall_segments, self.origin(), self.size());
        }

        let ppg = self.pixels_per_grid() as f64;
        line_of_sight_polygon.exterior_mut(|f| {
            f.coords_mut().for_each(|f| {
                f.x = (f.x * ppg).round();
                f.y = (f.y * ppg).round();
            })
        });
        self.fog_of_war.update(operation, &line_of_sight_polygon);

        Ok(())
    }

    /// Apply the current fog of war to the image, painting every fog of war covered pixel black
    /// and returning the updated image
    fn apply_fow(&self, image: &DynamicImage) -> RgbaImage {
        let mut image = image.to_rgba8();
        let rectangles = self.fog_of_war.get_rectangles();
        for rectangle in rectangles {
            rectangle.for_each_pixel(&mut |x, y| {
                if x < image.width() && y < image.height() {
                    image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                }
            });
        }
        image
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

    /// Apply all vtt data (fog of war, lighting, etc.) to the image stored in this vtt and save it to a .png file.
    pub fn save_img<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let decoded = BASE64_STANDARD.decode(self.image.as_str())?;
        let img = ImageReader::new(Cursor::new(decoded))
            .with_guessed_format()?
            .decode()?;
        let img = self.apply_fow(&img);
        save_buffer(
            path,
            &img,
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn vtt_save_small_img() {
        let vtt = open_vtt("tests/resources/example4.dd2vtt")
            .expect("Could not open file the example4.dd2vtt");
        vtt.save_img_raw("tests/resources/small.png")
            .expect("Failed to save to png");
    }

    #[test]
    fn vtt_fow_hide_all() {
        let mut vtt = open_vtt("tests/resources/example4.dd2vtt")
            .expect("Could not open file the example4.dd2vtt");
        vtt.fow_hide_all();
        vtt.save_img("tests/resources/black.png")
            .expect("Could not save the image to png")
    }

    #[test]
    fn vtt_fow_direct_los() {
        let mut vtt = open_vtt("tests/resources/example4.dd2vtt")
            .expect("Could not open file the example4.dd2vtt");
        vtt.fow_hide_all();
        let pov = Coordinate { x: 4.0, y: 7.0 };
        vtt.fow_change(pov, false, Operation::SHOW)
            .expect("Could not update fow");
        vtt.save_img("tests/resources/los.png")
            .expect("Could not save the image to png")
    }
}

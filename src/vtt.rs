use crate::{
    errors::RustVttError,
    fog_of_war::{FogOfWar, Operation},
    helper::{self, create_polygon, distance, find_intersection},
    vector::Vector,
};
use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use geo::{
    orient::Direction, Area, BooleanOps, Contains, Coord, Distance, Euclidean, Line, LineString,
    MultiPolygon, Orient, Polygon,
};
use image::{save_buffer, DynamicImage, ExtendedColorType, ImageReader, Rgb, RgbImage};
use imageproc::drawing;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    f64,
    fs::{File, OpenOptions},
    io::{Cursor, Write},
    path::Path,
};

const STEP_SIZE: f64 = 0.2;

/// A VTT struct containing all data that is in the .vtt file without fog of war.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VTTPartial {
    format: f32,
    resolution: Resolution,
    line_of_sight: Vec<Vec<Coordinate>>,
    objects_line_of_sight: Option<Vec<Vec<Coordinate>>>,
    portals: Vec<Portal>,
    environment: Environment,
    lights: Vec<Light>,
    image: String,
}

/// The main VTT structure containing all the data that is in the .vtt file including a fog of war
/// field.
#[derive(Debug, Clone)]
pub struct VTT {
    format: f32,
    resolution: Resolution,
    line_of_sight: Vec<Vec<Coordinate>>,
    objects_line_of_sight: Option<Vec<Vec<Coordinate>>>,
    portals: Vec<Portal>,
    environment: Environment,
    lights: Vec<Light>,
    fog_of_war: FogOfWar,
    image: String,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialOrd)]
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

impl PixelCoordinate {
    pub fn as_coord(self) -> Coord {
        Coord {
            x: self.x as f64,
            y: self.y as f64,
        }
    }
}

impl Coordinate {
    pub fn from_coord(coord: Coord) -> Self {
        Self {
            x: coord.x,
            y: coord.y,
        }
    }

    pub fn as_coord(self) -> Coord {
        Coord {
            x: self.x,
            y: self.y,
        }
    }

    /// Returns whether self is within one square of other
    pub fn within_square(&self, other: &Coordinate) -> bool {
        if other.x < self.x - 1. || self.x + 1. < other.x {
            return false;
        }
        if other.y < self.y - 1. || self.y + 1. < other.y {
            return false;
        }
        true
    }
}

impl PartialEq for Coordinate {
    fn eq(&self, other: &Self) -> bool {
        if self.x.is_nan() || other.x.is_nan() || self.y.is_nan() || other.y.is_nan() {
            panic!("Coordinate may not be NaN, undefined behaviour");
        }
        (self.x - other.x).abs() < 1e-9 && (self.y - other.y).abs() < 1e-9
    }
}

impl Eq for Coordinate {}

impl Ord for Coordinate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if (self.x - other.x) < 1e-9 {
            if (self.y - other.y) < 1e-9 {
                return Ordering::Equal;
            } else if self.y < other.y {
                return Ordering::Less;
            } else {
                return Ordering::Greater;
            }
        } else if self.x < other.x {
            return Ordering::Less;
        } else {
            return Ordering::Greater;
        }
    }
}

impl VTTPartial {
    /// Convert the partial vtt to a vtt struct that contains fog of war
    pub fn to_vtt(self) -> VTT {
        assert!(
            self.resolution.map_origin.x >= 0.0,
            "Origin x must positive"
        );
        assert!(
            self.resolution.map_origin.y >= 0.0,
            "Origin y must be positive"
        );
        assert_eq!(
            self.resolution.map_size.x.fract(),
            0.0,
            "The size must be a whole number"
        );
        assert_eq!(
            self.resolution.map_size.y.fract(),
            0.0,
            "The size must be a whole number"
        );
        let fog_of_war = FogOfWar::new(self.resolution);
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

    /// Open a door at the specified position. The position does not have to be exact but should be
    /// within one square of the position of a door. If multiple doors are within one square it
    /// will pick the door closest to the given position. Returns whether a was door found at the given
    /// position.
    pub fn open_door(&mut self, position: Coordinate) -> bool {
        let closest_door = self.portals.iter_mut().min_by(|x, y| {
            let dx = distance(&x.position.as_coord(), &position.as_coord());
            let dy = distance(&y.position.as_coord(), &position.as_coord());
            dx.total_cmp(&dy)
        });
        if let Some(door) = closest_door {
            if door.position.within_square(&position) {
                door.closed = false;
                return true;
            }
            return false;
        }
        false
    }

    /// Close a door at the specified position. The position does not have to be exact but should be
    /// within one square of the position of a door. If multiple doors are within one square it
    /// will pick the door closest to the given position. Returns whether a door was found at the given
    /// position.
    pub fn close_door(&mut self, position: Coordinate) -> bool {
        let closest_door = self.portals.iter_mut().min_by(|x, y| {
            let dx = distance(&x.position.as_coord(), &position.as_coord());
            let dy = distance(&y.position.as_coord(), &position.as_coord());
            dx.total_cmp(&dy)
        });
        if let Some(door) = closest_door {
            if door.position.within_square(&position) {
                door.closed = true;
                return true;
            }
            return false;
        }
        false
    }

    /// Apply ambient light and other light sources to given image
    fn apply_light(&self, image: &DynamicImage) -> RgbImage {
        todo!("apply light sources to image");
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
    /// standing at this coordinate could see.
    /// ## `pov`
    /// The coordinate at which the person you want to reveal area for is standing
    /// ## `around_walls`
    /// Whether the person at the pov point can look around walls perfectly. When false, this will
    /// function as a 'line of sight' fog of war update.
    /// ## `through_objects`
    /// Whether to let the vision go through objects defined in objects_line_of_sight
    pub fn fow_change(
        &mut self,
        pov: Coordinate,
        operation: Operation,
        around_walls: bool,
        through_objects: bool,
    ) -> Result<(), RustVttError> {
        // First check if the given coordinate is not on or out of the bounds of the grid
        if pov.x <= self.origin().x || self.size().x <= pov.x {
            return Err(RustVttError::OutOfBounds { coordinate: pov });
        }
        if pov.y <= self.origin().y || self.size().y <= pov.y {
            return Err(RustVttError::OutOfBounds { coordinate: pov });
        }
        // Check if the coordinate is not on a wall line
        let walls = self.get_line_segments(!through_objects);
        let pov_coord: Coord = pov.as_coord();
        for wall in &walls {
            if Euclidean::distance(wall, pov_coord) < 1e-9 {
                return Err(RustVttError::InvalidPoint { coordinate: pov });
            }
        }

        let mut line_of_sight_polygon: Polygon;
        if around_walls {
            line_of_sight_polygon = self.calculate_indirect_los(pov, &walls)
        } else {
            line_of_sight_polygon = self.calculate_direct_los(pov, &walls);
        }

        let ppg = self.pixels_per_grid() as f64;
        line_of_sight_polygon.exterior_mut(|f| {
            f.coords_mut().for_each(|f| {
                f.x = (f.x * ppg).round();
                f.y = (f.y * ppg).round();
            });
        });
        line_of_sight_polygon.interiors_mut(|r| {
            r.iter_mut().for_each(|l| {
                l.coords_mut().for_each(|c| {
                    c.x = (c.x * ppg).round();
                    c.y = (c.y * ppg).round();
                });
            });
        });

        self.fog_of_war.update(operation, &line_of_sight_polygon);

        Ok(())
    }

    /// Get all lines in the vtt that block line of sight. Any line segment with multiple
    /// coordinates will be split into seperate lines for streamlined formatting. Portals will be
    /// included if their closed field is true.
    /// ## `objects`
    /// Whether to include 'objects_line_of_sight' in the result
    /// ## panics
    /// if a portal is closed but has no bounds
    pub fn get_line_segments(&self, objects: bool) -> Vec<Line> {
        let mut all_lines: Vec<Line> = Vec::new();

        for line in &self.line_of_sight {
            let mut prev_point: Option<Coord> = None;
            for point in line {
                if let Some(prev) = prev_point {
                    all_lines.push(Line::new(prev, point.as_coord()));
                }
                prev_point = Some(point.as_coord());
            }
        }

        for portal in &self.portals {
            if portal.closed {
                let start = portal
                    .bounds
                    .get(0)
                    .expect("expected an start bound for portal");
                let end = portal
                    .bounds
                    .get(1)
                    .expect("expected an end bound for portal");
                all_lines.push(Line::new(start.as_coord(), end.as_coord()));
            }
        }

        if !objects {
            return all_lines;
        }
        let objects_line_of_sight = match &self.objects_line_of_sight {
            Some(o) => o,
            None => return all_lines,
        };

        for line in objects_line_of_sight {
            let mut prev_point: Option<Coord> = None;
            for point in line {
                if let Some(prev) = prev_point {
                    all_lines.push(Line::new(prev, point.as_coord()));
                }
                prev_point = Some(point.as_coord());
            }
        }

        all_lines
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
            ExtendedColorType::Rgb8,
        )?;
        Ok(())
    }

    /// Try to save the current vtt struct to the specified path, will overwrite the file if it
    /// already existed. This will not save fog of war state
    pub fn save_vtt<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut f = OpenOptions::new().write(true).truncate(true).open(path)?;
        let json_string = serde_json::to_string(&self.to_partialvtt())?;
        f.write_all(json_string.as_bytes())?;
        Ok(())
    }

    /**
     *
     *
     * ----------------------- Private functions -----------------------
     *
     *
     **/

    /// Apply the current fog of war to the image, painting every fog of war covered pixel black
    /// and returning the updated image
    fn apply_fow(&self, image: &DynamicImage) -> RgbImage {
        let mut image = image.to_rgb8();
        let rectangles: Vec<imageproc::rect::Rect> = self
            .fog_of_war
            .get_rectangles()
            .par_iter_mut()
            .map(|x| x.as_rect())
            .collect();
        for rect in rectangles {
            drawing::draw_filled_rect_mut(&mut image, rect, Rgb([0, 0, 0]));
        }
        image
    }

    /// Convert to a vtt partial struct which does not contain fog of war
    fn to_partialvtt(&self) -> VTTPartial {
        VTTPartial {
            format: self.format,
            resolution: self.resolution,
            line_of_sight: self.line_of_sight.clone(),
            objects_line_of_sight: self.objects_line_of_sight.clone(),
            portals: self.portals.clone(),
            environment: self.environment.clone(),
            lights: self.lights.clone(),
            image: self.image.clone(),
        }
    }

    /// Generate a Polygon representing the area that the pov can see. This vision is
    /// blocked by walls
    fn calculate_direct_los(&self, pov: Coordinate, walls: &Vec<Line>) -> Polygon {
        let mut intersections: Vec<Coord> = Vec::new();
        self.for_each_interesection(pov, 0, walls, &mut |intersection| {
            intersections.push(intersection.expect("skip 0 cannot result in None value"));
            false
        });
        let first = intersections.first().expect("No intersection found");
        let last = intersections.last().expect("No intersection found");
        // Make sure the ring is closed
        if distance(first, last) > 1e-9 {
            intersections.push(first.clone());
        }
        assert!(
            intersections.len() > 2,
            "Not enough intersections to form a linestring"
        );
        let linestring = LineString::new(intersections);
        assert!(
        linestring.is_closed(),
        "The resulting line of sight ring is not closed (Begin and end coordinate are not equal)"
        );
        Polygon::new(linestring, vec![])
    }

    /// Calculate the indirect line of sight following paths along walls
    fn calculate_indirect_los(&self, pov: Coordinate, walls: &Vec<Line>) -> Polygon {
        let mut walls_and_edges = walls.to_vec();
        let topleft = self.origin().as_coord();
        let topright = Coord {
            x: self.size().x,
            y: self.origin().y,
        };
        let bottomleft = Coord {
            x: self.origin().x,
            y: self.size().y,
        };
        let bottomright = self.size().as_coord();
        let topline = Line::new(topleft, topright);
        let rightline = Line::new(topright, bottomright);
        let bottomline = Line::new(bottomright, bottomleft);
        let leftline = Line::new(bottomleft, topleft);
        walls_and_edges.push(topline);
        walls_and_edges.push(rightline);
        walls_and_edges.push(bottomline);
        walls_and_edges.push(leftline);
        let planar_graph = helper::planar_graph(&walls_and_edges);
        let mut unhandled_vectors = planar_graph.to_vec();
        let mut found_polygons: Vec<Polygon> = Vec::new();
        let mut los_polygons: Vec<Polygon> = Vec::new();
        while !unhandled_vectors.is_empty() {
            let polygon = create_polygon(&planar_graph, &mut unhandled_vectors);
            if polygon.contains(&pov.as_coord()) {
                los_polygons.push(polygon);
            } else {
                found_polygons.push(polygon);
            }
        }
        let mut los_polygon = los_polygons
            .iter()
            .min_by(|x, y| x.unsigned_area().total_cmp(&y.unsigned_area()))
            .expect("Should be at least 1 element")
            .clone();
        for polygon in found_polygons {
            let multi_polygon = los_polygon.difference(&polygon);
            multi_polygon.into_iter().for_each(|p| {
                if p.contains(&pov.as_coord()) {
                    los_polygon = p
                }
            });
        }
        los_polygon
    }

    /// Run a closure for each intersection point from pov to the edge of a map, skip first 'skip'
    /// closest intersections. Intersections are given in a clockwise direction starting from
    /// the pov the the map origin. The closure should return a boolean: exit early (true) or
    /// continue normally (false)
    fn for_each_interesection<F: FnMut(Option<Coord>) -> bool>(
        &self,
        pov: Coordinate,
        skip: usize,
        walls: &Vec<Line>,
        f: &mut F,
    ) {
        // we do not loop through floats due to inaccuracies in floating point arithmetic
        // In the first loop we vary x and make a line for pov to the top and bottom of the map
        let x_min = self.origin().x as i32;
        let x_max = self.size().x as i32;
        let y_min = self.origin().y;
        let start = Coord { x: pov.x, y: pov.y };
        // Line from pov to top edge
        for x in x_min..=(x_max * (1.0 / STEP_SIZE) as i32) {
            let x = f64::from(x) * STEP_SIZE;
            let end = Coord { x, y: y_min };
            let line = Line::new(start, end);
            let intersection = find_intersection(&line, walls, skip);
            if f(intersection) {
                return;
            }
        }
        let x_max = self.size().x;
        let y_min = self.origin().y as i32 + 1;
        let y_max = self.size().y as i32;
        // Line from pov to right edge
        for y in y_min..=(y_max * (1.0 / STEP_SIZE) as i32) {
            let y = f64::from(y) * STEP_SIZE;
            let end = Coord { x: x_max, y };
            let line = Line::new(start, end);
            let intersection = find_intersection(&line, walls, skip);
            if f(intersection) {
                return;
            }
        }
        // Line from pov to bottom edge
        let x_min = self.origin().x as i32;
        let x_max = self.size().x as i32;
        let y_max = self.size().y;
        for x in (x_min..=((x_max * (1.0 / STEP_SIZE) as i32) - 1)).rev() {
            let x = f64::from(x) * STEP_SIZE;
            let end = Coord { x, y: y_max };
            let line = Line::new(start, end);
            let intersection = find_intersection(&line, walls, skip);
            if f(intersection) {
                return;
            }
        }
        // Line from pov to left edge
        let x_max = self.origin().x;
        let y_min = self.origin().y as i32;
        let y_max = self.size().y as i32;
        for y in (y_min..=((y_max * (1.0 / STEP_SIZE) as i32) - 1)).rev() {
            let y = f64::from(y) * STEP_SIZE;
            let end = Coord { x: x_max, y };
            let line = Line::new(start, end);
            let intersection = find_intersection(&line, walls, skip);
            if f(intersection) {
                return;
            }
        }
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
        let vtt =
            open_vtt("tests/resources/tavern.uvtt").expect("Could not open file the tavern.uvtt");
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
        vtt.fow_change(pov, Operation::SHOW, false, true)
            .expect("Could not update fow");
        vtt.save_img("tests/resources/los.png")
            .expect("Could not save the image to png")
    }
}

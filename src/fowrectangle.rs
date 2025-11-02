use crate::errors::RustVttError;
use crate::quadtreenode::InLineString;
use crate::vtt::PixelCoordinate;
use crate::vtt::Resolution;
use geo::Area;
use geo::BooleanOps;
use geo::Coord;
use geo::Polygon;
use geo::Rect as georect;
use imageproc::rect::Rect as imageprocrect;

/// should not be smaller then 3
const MIN_SQUARE_SIZE: i32 = 3;

// One rectangle within the quad tree represented by 4 corner nodes
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct FoWRectangle {
    pub topleft: PixelCoordinate,
    pub bottomright: PixelCoordinate,
}

impl FoWRectangle {
    /// Create a new rectangle from a start and end point
    pub fn new(topleft: PixelCoordinate, bottomright: PixelCoordinate) -> Self {
        Self {
            topleft,
            bottomright,
        }
    }

    /// create a rectangle from the map resolution, this function creates the initial root square
    /// in the quad tree.
    pub fn from_resolution(resolution: &Resolution) -> Self {
        Self {
            topleft: PixelCoordinate::from(&resolution.map_origin, resolution.pixels_per_grid),
            bottomright: PixelCoordinate::from(&resolution.map_size, resolution.pixels_per_grid),
        }
    }

    /// Checks whether the current rectangle is inside the polygon, but not inside any interior
    /// linestrings
    pub fn in_polygon(&self, polygon: &Polygon) -> InLineString {
        let rectangle = self.to_rectangle().to_polygon();
        let exterior_intersection = polygon.intersection(&rectangle).unsigned_area();
        let rectangle_area = rectangle.unsigned_area();
        if (exterior_intersection / rectangle_area) > 0.9999 {
            return InLineString::INSIDE;
        }
        if (exterior_intersection / rectangle_area) < 0.0001 {
            return InLineString::OUTSIDE;
        }
        InLineString::PARTIAL
    }

    /// Turn FowRectangle into a geo::Rect
    fn to_rectangle(&self) -> geo::Rect {
        let min: Coord = self.topleft.as_coord();
        let max: Coord = self.bottomright.as_coord();
        georect::new(min, max)
    }

    pub fn as_rect(&self) -> imageproc::rect::Rect {
        let x: i32 = self.topleft.x;
        let y: i32 = self.topleft.y;
        let width = (self.bottomright.x - self.topleft.x) as u32 + 1;
        let height = (self.bottomright.y - self.topleft.y) as u32 + 1;
        imageprocrect::at(x, y).of_size(width, height)
    }

    /// Splits the given rectangle into four equally sized rectangles
    pub fn split(
        &self,
    ) -> Result<(FoWRectangle, FoWRectangle, FoWRectangle, FoWRectangle), RustVttError> {
        let width = self.bottomright.x - self.topleft.x;
        let height = self.bottomright.y - self.topleft.y; // pixels count up from top to bottom of
                                                          // the screen
        if width < MIN_SQUARE_SIZE || height < MIN_SQUARE_SIZE {
            return Err(RustVttError::MinimumRectangle {
                rectangle: self.clone(),
            });
        }
        let topleft_child = FoWRectangle {
            topleft: self.topleft,
            bottomright: PixelCoordinate::new(
                self.topleft.x + (width / 2),
                self.topleft.y + (height / 2),
            ),
        };
        let topright_child = FoWRectangle {
            topleft: PixelCoordinate::new(self.topleft.x + (width / 2) + 1, self.topleft.y),
            bottomright: PixelCoordinate::new(self.bottomright.x, self.topleft.y + (height / 2)),
        };
        let bottomleft_child = FoWRectangle {
            topleft: PixelCoordinate::new(self.topleft.x, self.topleft.y + (height / 2) + 1),
            bottomright: PixelCoordinate::new(self.topleft.x + (width / 2), self.bottomright.y),
        };
        let bottomright_child = FoWRectangle {
            topleft: PixelCoordinate::new(
                self.topleft.x + (width / 2) + 1,
                self.topleft.y + (height / 2) + 1,
            ),
            bottomright: self.bottomright,
        };
        Ok((
            topleft_child,
            topright_child,
            bottomleft_child,
            bottomright_child,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_normal_rectangle() {
        let rect = FoWRectangle {
            topleft: PixelCoordinate::new(0, 0),
            bottomright: PixelCoordinate::new(10, 10),
        };

        let (tl, tr, bl, br) = rect.split().expect("split should succeed");

        // Top-left child
        assert_eq!(
            tl,
            FoWRectangle {
                topleft: PixelCoordinate::new(0, 0),
                bottomright: PixelCoordinate::new(5, 5),
            }
        );

        // Top-right child
        assert_eq!(
            tr,
            FoWRectangle {
                topleft: PixelCoordinate::new(6, 0),
                bottomright: PixelCoordinate::new(10, 5),
            }
        );

        // Bottom-left child
        assert_eq!(
            bl,
            FoWRectangle {
                topleft: PixelCoordinate::new(0, 6),
                bottomright: PixelCoordinate::new(5, 10),
            }
        );

        // Bottom-right child
        assert_eq!(
            br,
            FoWRectangle {
                topleft: PixelCoordinate::new(6, 6),
                bottomright: PixelCoordinate::new(10, 10),
            }
        );
    }

    #[test]
    fn split_odd_size_rectangle() {
        // width and height are 11 pixels
        let rect = FoWRectangle {
            topleft: PixelCoordinate::new(0, 0),
            bottomright: PixelCoordinate::new(11, 11),
        };

        let (tl, tr, bl, _br) = rect.split().expect("split should succeed");

        // Verify that the resulting rectangles are roughly equal size
        let width_tl = tl.bottomright.x - tl.topleft.x;
        let width_tr = tr.bottomright.x - tr.topleft.x;
        let height_tl = tl.bottomright.y - tl.topleft.y;
        let height_bl = bl.bottomright.y - bl.topleft.y;

        assert!(
            (width_tl - width_tr).abs() <= 1,
            "Widths differ by more than 1: {width_tl} vs {width_tr}"
        );
        assert!(
            (height_tl - height_bl).abs() <= 1,
            "Heights differ by more than 1: {height_tl} vs {height_bl}"
        );
    }

    #[test]
    fn split_minimum_size_error() {
        // Only 1 pixel wide/high
        let rect = FoWRectangle {
            topleft: PixelCoordinate::new(0, 0),
            bottomright: PixelCoordinate::new(1, 1),
        };

        let result = rect.split();
        match result {
            Err(RustVttError::MinimumRectangle { rectangle }) => {
                assert_eq!(rectangle.topleft, PixelCoordinate::new(0, 0));
            }
            other => panic!("Expected MinimumRectangle error, got {:?}", other),
        }
    }
}

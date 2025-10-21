use crate::vtt::Coordinate;
use crate::vtt::PixelCoordinate;
use crate::vtt::Resolution;
pub enum QuadtreeNode {
    Leaf {
        rectangle: FoWRectangle,
    },
    Internal {
        topleft: Box<QuadtreeNode>,
        topright: Box<QuadtreeNode>,
        bottomleft: Box<QuadtreeNode>,
        bottomright: Box<QuadtreeNode>,
    },
}

// One rectangle within the quad tree represented by 4 corner nodes
pub struct FoWRectangle {
    topleft: PixelCoordinate,
    topright: PixelCoordinate,
    bottomleft: PixelCoordinate,
    bottomright: PixelCoordinate,
}

impl FoWRectangle {
    /// create a rectangle from the map resolution, this function creates the initial root square
    /// in the quad tree.
    pub fn from_resolution(resolution: &Resolution) -> Self {
        Self {
            topleft: PixelCoordinate::from(&resolution.map_origin, resolution.pixels_per_grid),
            topright: PixelCoordinate::from(
                &Coordinate {
                    x: resolution.map_size.x,
                    y: resolution.map_origin.y,
                },
                resolution.pixels_per_grid,
            ),
            bottomleft: PixelCoordinate::from(
                &Coordinate {
                    x: resolution.map_origin.x,
                    y: resolution.map_size.y,
                },
                resolution.pixels_per_grid,
            ),
            bottomright: PixelCoordinate::from(&resolution.map_size, resolution.pixels_per_grid),
        }
    }
}

impl Default for FoWRectangle {
    fn default() -> Self {
        Self {
            topleft: PixelCoordinate { x: 0, y: 0 },
            topright: PixelCoordinate { x: 0, y: 0 },
            bottomleft: PixelCoordinate { x: 0, y: 0 },
            bottomright: PixelCoordinate { x: 0, y: 0 },
        }
    }
}

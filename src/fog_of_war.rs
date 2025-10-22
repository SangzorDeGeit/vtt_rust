//! The FogOfWar is a quadtree that efficiently stores information on which pixels in the image are
//! covered by fog of war. This struct is used in the VTT struct. It is generally recommended to
//! access this struct via the VTT implementation.

use geo::LineString;

use crate::quadtreenode::{FoWRectangle, QuadtreeNode};
use crate::vtt::Resolution;

/// A quadtree representing fog of war
#[derive(Debug, Clone)]
pub struct FogOfWar {
    resolution: Resolution,
    root: QuadtreeNode,
}

pub enum Operation {
    HIDE,
    SHOW,
}

impl FogOfWar {
    /// Creates a new fog of war with the maximum coverage equal to the resolution
    pub fn new(resolution: Resolution) -> Self {
        let bounds = FoWRectangle::from_resolution(&resolution);
        Self {
            resolution,
            root: QuadtreeNode::Leaf {
                bounds,
                visible: true,
            },
        }
    }

    /// Set the fog of war area to hide everyting
    pub fn hide_all(&mut self) {
        self.root = QuadtreeNode::Leaf {
            bounds: FoWRectangle::from_resolution(&self.resolution),
            visible: false,
        }
    }

    /// Set the fog of war area to visible
    pub fn show_all(&mut self) {
        self.root = QuadtreeNode::Leaf {
            bounds: FoWRectangle::from_resolution(&self.resolution),
            visible: true,
        }
    }

    /// Update the fog of war according to a given polygon
    pub fn update(&mut self, operation: Operation, polygon: &LineString) {
        let mut new_area = match &operation {
            Operation::HIDE => QuadtreeNode::from_resolution(&self.resolution, true),
            Operation::SHOW => QuadtreeNode::from_resolution(&self.resolution, false),
        };
        match &operation {
            Operation::HIDE => {
                new_area.create_tree(false, polygon);
                self.root.hide(&new_area);
            }
            Operation::SHOW => {
                new_area.create_tree(true, polygon);
                self.root.show(&new_area);
            }
        }
    }

    /// Gets all rectangles covered by fog of war
    pub fn get_rectangles(&self) -> Vec<FoWRectangle> {
        let mut vec = Vec::new();
        self.root.populate_rectangle_vec(&mut vec);
        vec
    }
}

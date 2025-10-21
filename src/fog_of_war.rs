//! The FogOfWar is a quadtree that efficiently stores information on which pixels in the image are
//! covered by fog of war. This struct is used in the VTT struct and should generally only be accessed
//! via the VTT struct.

use geo::LineString;

use crate::quadtreenode::{FoWRectangle, QuadtreeNode};
use crate::vtt::Resolution;

/// A quadtree representing fog of war
pub struct FogOfWar {
    root: QuadtreeNode,
}

pub enum Operation {
    HIDE,
    SHOW,
}

impl FogOfWar {
    /// Creates a new fog of war with the maximum coverage equal to the resolution
    pub fn new(resolution: &Resolution) -> Self {
        Self {
            root: QuadtreeNode::Leaf {
                rectangle: FoWRectangle::from_resolution(resolution),
            },
        }
    }
    /// Set the fog of war area to the entire resolution of the screen
    pub fn hide_all(&mut self, resolution: &Resolution) {
        self.root = QuadtreeNode::Leaf {
            rectangle: FoWRectangle::from_resolution(resolution),
        }
    }

    pub fn update(&mut self, operation: Operation, polygon: &LineString) {}
}

impl QuadtreeNode {}

// Create test for the from_fowsquare

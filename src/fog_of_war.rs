//! The FogOfWar is an array of squares equal to the amount of grid squares. Each grid square is either
//! completely visible, completely invisible or the root of a mini quadtree representing what part
//! of the square is visible. Normally this struct is accessed via the VTT implementation, but you
//! can also use this struct directly for more control over the fog of war.

use geo::Polygon;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use crate::quadtreenode::{FoWRectangle, InLineString, QuadtreeNode};
use crate::vtt::{PixelCoordinate, Resolution};

#[derive(Debug, Clone)]
pub struct FogOfWar {
    squares: Vec<FowNode>,
}

#[derive(Debug, Clone)]
pub struct FowNode {
    state: FowState,
    rect: FoWRectangle,
}

#[derive(Debug, Clone)]
pub enum FowState {
    Hidden,
    Shown,
    Partial { node: QuadtreeNode },
}

impl Default for FowState {
    fn default() -> Self {
        Self::Shown
    }
}

pub enum Operation {
    HIDE,
    SHOW,
}

impl FogOfWar {
    /// Create a new fog of war area with size equal to the resolution
    pub fn new(resolution: Resolution) -> Self {
        let pixel_origin =
            PixelCoordinate::from(&resolution.map_origin, resolution.pixels_per_grid);
        let pixel_size = PixelCoordinate::from(&resolution.map_size, resolution.pixels_per_grid);
        let mut squares: Vec<FowNode> = Vec::with_capacity(
            (resolution.map_size.x.ceil() * resolution.map_size.y.ceil()) as usize,
        );
        let mut x = pixel_origin.x;
        let mut y = pixel_origin.y;
        while y < pixel_size.y {
            while x < pixel_size.x {
                let topleft = PixelCoordinate::new(x, y);
                x += resolution.pixels_per_grid - 1;
                let bottomright = PixelCoordinate::new(x, y + resolution.pixels_per_grid - 1);
                let node = FowNode::new(FoWRectangle::new(topleft, bottomright));
                squares.push(node);
                x += 1;
            }
            x = pixel_origin.x;
            y += resolution.pixels_per_grid;
        }
        Self { squares }
    }

    /// Set the fog of war area to hide everyting
    pub fn hide_all(&mut self) {
        self.squares.iter_mut().for_each(|f| f.hide());
    }

    /// Set the fog of war area to visible
    pub fn show_all(&mut self) {
        self.squares.iter_mut().for_each(|f| f.show());
    }

    /// Update the fog of war according to a given polygon
    pub fn update(&mut self, operation: Operation, polygon: &Polygon) {
        let make_visible = match operation {
            Operation::HIDE => false,
            Operation::SHOW => true,
        };
        self.squares
            .par_iter_mut()
            .for_each(|f| f.update(polygon, make_visible));
    }

    /// Gets all rectangles covered by fog of war
    pub fn get_rectangles(&self) -> Vec<FoWRectangle> {
        let vec: Vec<Vec<FoWRectangle>> = self.squares.iter().map(|f| f.rectangles()).collect();
        let vec: Vec<FoWRectangle> = vec.into_iter().flatten().collect();
        vec
    }
}

impl FowNode {
    /// Create a new node with area equal to the given rectangle
    pub fn new(rect: FoWRectangle) -> Self {
        Self {
            state: FowState::Shown,
            rect,
        }
    }

    /// Sets the state of the current node to hidden
    pub fn hide(&mut self) {
        self.state = FowState::Hidden;
    }

    /// Sets the state of the current node to shown
    pub fn show(&mut self) {
        self.state = FowState::Shown;
    }

    /// Update this node according to the polygon and if the polygon makes areas visible
    /// Example: if make_visible is false the polygon represents addition of fog of war
    pub fn update(&mut self, polygon: &Polygon, make_visible: bool) {
        use InLineString as I;
        match self.rect.in_polygon(polygon) {
            I::INSIDE => {
                if make_visible {
                    self.show();
                } else {
                    self.hide();
                }
            }
            I::OUTSIDE => (),
            I::PARTIAL => {
                self.partial(make_visible, polygon);
            }
        }
    }

    /// Sets the state of the current node to partial and sets the quadtree according to a given
    /// polygon and visibility
    pub fn partial(&mut self, make_visible: bool, polygon: &Polygon) {
        let mut quad_tree = QuadtreeNode::from_bounds(self.rect, !make_visible);
        quad_tree.create_tree(make_visible, &polygon);
        match &mut self.state {
            FowState::Partial { node } => {
                if make_visible {
                    node.show(&quad_tree);
                    node.clean();
                } else {
                    node.hide(&quad_tree);
                    node.clean();
                }
            }
            _ => {
                self.state = FowState::Partial { node: quad_tree };
            }
        }
    }

    /// Return all rectangles covered by fog of war in this node
    pub fn rectangles(&self) -> Vec<FoWRectangle> {
        let mut rectangle_vec: Vec<FoWRectangle> = Vec::new();
        match &self.state {
            FowState::Partial { node } => node.populate_rectangle_vec(&mut rectangle_vec),
            FowState::Hidden => rectangle_vec.push(self.rect),
            FowState::Shown => (),
        }
        rectangle_vec
    }
}

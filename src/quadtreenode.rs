use geo::Contains;
use geo::Coord;
use geo::Intersects;
use geo::LineString;
use geo::Rect;

use crate::errors::RustVttError;
use crate::fog_of_war::Operation;
use crate::vtt::Coordinate;
use crate::vtt::PixelCoordinate;
use crate::vtt::Resolution;
#[derive(Debug, Clone)]
pub enum QuadtreeNode {
    Leaf {
        bounds: FoWRectangle,
        visible: bool,
    },
    Internal {
        topleft: Box<QuadtreeNode>,
        topright: Box<QuadtreeNode>,
        bottomleft: Box<QuadtreeNode>,
        bottomright: Box<QuadtreeNode>,
    },
}

// One rectangle within the quad tree represented by 4 corner nodes
#[derive(Debug, Clone, PartialEq)]
pub struct FoWRectangle {
    topleft: PixelCoordinate,
    topright: PixelCoordinate,
    bottomleft: PixelCoordinate,
    bottomright: PixelCoordinate,
}

pub enum InLineString {
    INSIDE,
    OUTSIDE,
    PARTIAL,
}

impl QuadtreeNode {
    /// Creates a new leaf node with a fowrectangle
    pub fn from_bounds(bounds: FoWRectangle, visible: bool) -> Self {
        Self::Leaf { bounds, visible }
    }

    /// Creates a new leaf node with a resolution
    pub fn from_resolution(resolution: &Resolution, visible: bool) -> Self {
        Self::Leaf {
            bounds: FoWRectangle::from_resolution(resolution),
            visible,
        }
    }

    /// Converts a node into an internal node, does nothing if the given node is already internal.
    /// If the node is a leaf node it will split the bounding box into four rectangles, this
    /// function returns an error if the rectangle is already the minimum size
    pub fn to_internal(&mut self) -> Result<(), RustVttError> {
        let children = match self {
            QuadtreeNode::Leaf { bounds, .. } => bounds.split()?,
            QuadtreeNode::Internal { .. } => return Ok(()),
        };
        let topleft = Box::new(Self::from_bounds(children.0, true));
        let topright = Box::new(Self::from_bounds(children.1, true));
        let bottomleft = Box::new(Self::from_bounds(children.2, true));
        let bottomright = Box::new(Self::from_bounds(children.3, true));
        *self = Self::Internal {
            topleft,
            topright,
            bottomleft,
            bottomright,
        };

        Ok(())
    }

    /// Get the area that this quadtree node should cover
    fn get_area(&self) -> FoWRectangle {
        let topleft = self.get_topleft_point();
        let topright = self.get_topright_point();
        let bottomleft = self.get_bottomleft_point();
        let bottomright = self.get_bottomright_point();
        FoWRectangle {
            topleft,
            topright,
            bottomleft,
            bottomright,
        }
    }

    /// Get topleft point of self
    fn get_topleft_point(&self) -> PixelCoordinate {
        match self {
            Self::Leaf { bounds, .. } => bounds.topleft,
            Self::Internal { topleft, .. } => topleft.get_topleft_point(),
        }
    }

    /// Get topright point of self
    fn get_topright_point(&self) -> PixelCoordinate {
        match self {
            QuadtreeNode::Leaf { bounds, .. } => bounds.topright,
            QuadtreeNode::Internal { topright, .. } => topright.get_topright_point(),
        }
    }

    /// Get bottomleft point of self
    fn get_bottomleft_point(&self) -> PixelCoordinate {
        match self {
            QuadtreeNode::Leaf { bounds, .. } => bounds.bottomleft,
            QuadtreeNode::Internal { bottomleft, .. } => bottomleft.get_bottomleft_point(),
        }
    }

    /// Get bottomright point of self
    fn get_bottomright_point(&self) -> PixelCoordinate {
        match self {
            QuadtreeNode::Leaf { bounds, .. } => bounds.bottomright,
            QuadtreeNode::Internal { bottomright, .. } => bottomright.get_bottomright_point(),
        }
    }

    /// Given a line of sight polygon and an operation this function will create a tree that
    /// reveals or hides the part of the polygon. The input to this function should be a completely
    /// hidden (visible=false) or completely shown (visible=true) root node.
    pub fn create_tree(&mut self, make_visible: bool, polygon: &LineString) {
        match self {
            Self::Leaf { bounds, visible } => match bounds.in_linestring(polygon) {
                InLineString::INSIDE => {
                    *visible = make_visible;
                }
                InLineString::OUTSIDE => {
                    *visible = !make_visible;
                }
                InLineString::PARTIAL => {
                    if let Err(_) = self.to_internal() {
                        return;
                    }
                    self.create_tree(make_visible, polygon);
                }
            },
            Self::Internal {
                topleft,
                topright,
                bottomleft,
                bottomright,
            } => {
                topleft.create_tree(make_visible, polygon);
                topright.create_tree(make_visible, polygon);
                bottomleft.create_tree(make_visible, polygon);
                bottomright.create_tree(make_visible, polygon);
                return;
            }
        };
        // for the node: if it is a leaf: check its relation to the polygon
        // hide or show it completely, or turn it into an internal node
        // if it is an internal node, follow each subnote
    }

    /// Add fog of war represented by other to self
    pub fn hide(&mut self, other: &Self) {
        use QuadtreeNode as Q;
        match (&mut *self, other) {
            (
                Q::Leaf {
                    visible: visible_self,
                    ..
                },
                Q::Leaf {
                    visible: visible_other,
                    ..
                },
            ) => {
                if *visible_self && !visible_other {
                    *visible_self = false;
                }
                return;
            }
            (Q::Leaf { visible, .. }, Q::Internal { .. }) => {
                if !*visible {
                    return;
                }
                self.to_internal()
                    .expect("expected self to be able to split");
                self.hide(other);
            }
            (Q::Internal { .. }, Q::Leaf { visible, .. }) => {
                if !visible {
                    *self = Self::Leaf {
                        bounds: self.get_area(),
                        visible: false,
                    };
                }
                return;
            }
            (
                Q::Internal {
                    topleft: tl_self,
                    topright: tr_self,
                    bottomleft: bl_self,
                    bottomright: br_self,
                },
                Q::Internal {
                    topleft: tl_other,
                    topright: tr_other,
                    bottomleft: bl_other,
                    bottomright: br_other,
                },
            ) => {
                tl_self.hide(tl_other);
                tr_self.hide(tr_other);
                bl_self.hide(bl_other);
                br_self.hide(br_other);
            }
        }
    }

    /// Remove fog of war represented by other from self
    pub fn show(&mut self, other: &Self) {
        use QuadtreeNode as Q;
        match (&mut *self, other) {
            (
                Q::Leaf {
                    visible: visible_self,
                    ..
                },
                Q::Leaf {
                    visible: visible_other,
                    ..
                },
            ) => {
                if !*visible_self && *visible_other {
                    *visible_self = true;
                }
                return;
            }
            (Q::Leaf { visible, .. }, Q::Internal { .. }) => {
                if *visible {
                    return;
                }
                self.to_internal()
                    .expect("expected self to be able to split");
                self.show(other);
            }
            (Q::Internal { .. }, Q::Leaf { visible, .. }) => {
                if *visible {
                    *self = Self::Leaf {
                        bounds: self.get_area(),
                        visible: true,
                    }
                }
                return;
            }
            (
                Q::Internal {
                    topleft: tl_self,
                    topright: tr_self,
                    bottomleft: bl_self,
                    bottomright: br_self,
                },
                Q::Internal {
                    topleft: tl_other,
                    topright: tr_other,
                    bottomleft: bl_other,
                    bottomright: br_other,
                },
            ) => {
                tl_self.show(tl_other);
                tr_self.show(tr_other);
                bl_self.show(bl_other);
                br_self.show(br_other);
            }
        }
    }

    /// Populates the given vec with rectangles from the tree representing fog of war (leaf nodes
    /// where visible=false)
    pub fn populate_rectangle_vec(&self, vec: &mut Vec<FoWRectangle>) {
        match self {
            QuadtreeNode::Leaf { bounds, visible } => {
                if !visible {
                    vec.push(bounds.clone());
                }
            }
            QuadtreeNode::Internal {
                topleft,
                topright,
                bottomleft,
                bottomright,
            } => {
                topleft.populate_rectangle_vec(vec);
                topright.populate_rectangle_vec(vec);
                bottomleft.populate_rectangle_vec(vec);
                bottomright.populate_rectangle_vec(vec);
            }
        }
    }
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

    /// Checks whether the current rectangle is inside the linestring by checking each corner point
    /// of the rectangle.
    pub fn in_linestring(&self, linestring: &LineString) -> InLineString {
        let rectangle = self.to_rectangle();
        if linestring.contains(&rectangle) {
            return InLineString::INSIDE;
        }
        if !rectangle.intersects(linestring) {
            return InLineString::OUTSIDE;
        }
        InLineString::PARTIAL
    }

    /// Turn FowRectangle into a geo::Rect
    fn to_rectangle(&self) -> Rect {
        let min: Coord = self.topleft.clone().into();
        let max: Coord = self.bottomright.clone().into();
        Rect::new(min, max)
    }

    /// Splits the given rectangle into four equally sized rectangles
    fn split(
        &self,
    ) -> Result<(FoWRectangle, FoWRectangle, FoWRectangle, FoWRectangle), RustVttError> {
        let width = self.topright.x - self.topleft.x;
        let height = self.bottomleft.y - self.topleft.y; // pixels count up from top to bottom of
                                                         // the screen
        if width < 2 || height < 2 {
            return Err(RustVttError::MinimumRectangle {
                rectangle: self.clone(),
            });
        }
        let topleft_child = FoWRectangle {
            topleft: self.topleft,
            topright: PixelCoordinate::new(self.topleft.x + (width / 2), self.topleft.y),
            bottomleft: PixelCoordinate::new(self.topleft.x, self.topleft.y + (height / 2)),
            bottomright: PixelCoordinate::new(
                self.topleft.x + (width / 2),
                self.topleft.y + (height / 2),
            ),
        };
        let topright_child = FoWRectangle {
            topleft: PixelCoordinate::new(self.topleft.x + (width / 2) + 1, self.topright.y),
            topright: self.topright,
            bottomleft: PixelCoordinate::new(
                self.topleft.x + (width / 2) + 1,
                self.topleft.y + (height / 2),
            ),
            bottomright: PixelCoordinate::new(self.topright.x, self.topleft.y + (height / 2)),
        };
        let bottomleft_child = FoWRectangle {
            topleft: PixelCoordinate::new(self.bottomleft.x, self.topleft.y + (height / 2) + 1),
            topright: PixelCoordinate::new(
                self.bottomleft.x + (width / 2),
                self.topleft.y + (height / 2) + 1,
            ),
            bottomleft: self.bottomleft,
            bottomright: PixelCoordinate::new(self.bottomleft.x + (width / 2), self.bottomleft.y),
        };
        let bottomright_child = FoWRectangle {
            topleft: PixelCoordinate::new(
                self.topleft.x + (width / 2) + 1,
                self.topleft.y + (height / 2) + 1,
            ),
            topright: PixelCoordinate::new(self.bottomright.x, self.topleft.y + (height / 2) + 1),
            bottomleft: PixelCoordinate::new(
                self.bottomleft.x + (width / 2) + 1,
                self.bottomright.y,
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

    /// Run a closure for each pixel in the rectangle
    pub fn for_each_pixel<F: FnMut(u32, u32)>(&self, f: &mut F) {
        for x in self.topleft.x..self.topright.x {
            for y in self.topleft.y..self.bottomleft.y {
                f(x as u32, y as u32)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_normal_rectangle() {
        let rect = FoWRectangle {
            topleft: PixelCoordinate::new(0, 0),
            topright: PixelCoordinate::new(10, 0),
            bottomleft: PixelCoordinate::new(0, 10),
            bottomright: PixelCoordinate::new(10, 10),
        };

        let (tl, tr, bl, br) = rect.split().expect("split should succeed");

        // Top-left child
        assert_eq!(
            tl,
            FoWRectangle {
                topleft: PixelCoordinate::new(0, 0),
                topright: PixelCoordinate::new(5, 0),
                bottomleft: PixelCoordinate::new(0, 5),
                bottomright: PixelCoordinate::new(5, 5),
            }
        );

        // Top-right child
        assert_eq!(
            tr,
            FoWRectangle {
                topleft: PixelCoordinate::new(6, 0),
                topright: PixelCoordinate::new(10, 0),
                bottomleft: PixelCoordinate::new(6, 5),
                bottomright: PixelCoordinate::new(10, 5),
            }
        );

        // Bottom-left child
        assert_eq!(
            bl,
            FoWRectangle {
                topleft: PixelCoordinate::new(0, 6),
                topright: PixelCoordinate::new(5, 6),
                bottomleft: PixelCoordinate::new(0, 10),
                bottomright: PixelCoordinate::new(5, 10),
            }
        );

        // Bottom-right child
        assert_eq!(
            br,
            FoWRectangle {
                topleft: PixelCoordinate::new(6, 6),
                topright: PixelCoordinate::new(10, 6),
                bottomleft: PixelCoordinate::new(6, 10),
                bottomright: PixelCoordinate::new(10, 10),
            }
        );
    }

    #[test]
    fn split_odd_size_rectangle() {
        // width and height are 11 pixels
        let rect = FoWRectangle {
            topleft: PixelCoordinate::new(0, 0),
            topright: PixelCoordinate::new(11, 0),
            bottomleft: PixelCoordinate::new(0, 11),
            bottomright: PixelCoordinate::new(11, 11),
        };

        let (tl, tr, bl, _br) = rect.split().expect("split should succeed");

        // Verify that the resulting rectangles are roughly equal size
        let width_tl = tl.topright.x - tl.topleft.x;
        let width_tr = tr.topright.x - tr.topleft.x;
        let height_tl = tl.bottomleft.y - tl.topleft.y;
        let height_bl = bl.bottomleft.y - bl.topleft.y;

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
            topright: PixelCoordinate::new(1, 0),
            bottomleft: PixelCoordinate::new(0, 1),
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

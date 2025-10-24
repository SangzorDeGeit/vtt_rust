use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use geo::Polygon;

use crate::errors::RustVttError;
use crate::fowrectangle::FoWRectangle;
use crate::vtt::PixelCoordinate;
use crate::vtt::Resolution;

pub enum InLineString {
    INSIDE,
    OUTSIDE,
    PARTIAL,
}

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
        let (children, visible) = match self {
            QuadtreeNode::Leaf { bounds, visible } => (bounds.split()?, visible),
            QuadtreeNode::Internal { .. } => return Ok(()),
        };
        let topleft = Box::new(Self::from_bounds(children.0, *visible));
        let topright = Box::new(Self::from_bounds(children.1, *visible));
        let bottomleft = Box::new(Self::from_bounds(children.2, *visible));
        let bottomright = Box::new(Self::from_bounds(children.3, *visible));
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
        let bottomright = self.get_bottomright_point();
        FoWRectangle {
            topleft,
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
    pub fn create_tree(
        &mut self,
        make_visible: bool,
        polygon: &Polygon,
        rect_counter: Arc<AtomicUsize>,
    ) {
        match self {
            Self::Leaf { bounds, visible } => match bounds.in_polygon(polygon) {
                InLineString::INSIDE => {
                    *visible = make_visible;
                    if *visible {
                        rect_counter.fetch_sub(1, Ordering::Relaxed);
                    } else {
                        rect_counter.fetch_add(1, Ordering::Relaxed);
                    }
                    return;
                }
                InLineString::OUTSIDE => {
                    *visible = !make_visible;
                    if *visible {
                        rect_counter.fetch_sub(1, Ordering::Relaxed);
                    } else {
                        rect_counter.fetch_add(1, Ordering::Relaxed);
                    }
                    return;
                }
                InLineString::PARTIAL => {
                    if let Err(_) = self.to_internal() {
                        return;
                    }
                    self.create_tree(make_visible, polygon, rect_counter);
                    return;
                }
            },
            Self::Internal {
                topleft,
                topright,
                bottomleft,
                bottomright,
            } => {
                topleft.create_tree(make_visible, polygon, rect_counter.clone());
                topright.create_tree(make_visible, polygon, rect_counter.clone());
                bottomleft.create_tree(make_visible, polygon, rect_counter.clone());
                bottomright.create_tree(make_visible, polygon, rect_counter.clone());
                return;
            }
        };
    }

    /// Add fog of war represented by other to self
    pub fn hide(&mut self, other: &Self, rect_counter: Arc<AtomicUsize>) {
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
                    rect_counter.fetch_add(1, Ordering::Relaxed);
                }
                return;
            }
            (Q::Leaf { visible, .. }, Q::Internal { .. }) => {
                if !*visible {
                    return;
                }
                self.to_internal()
                    .expect("expected self to be able to split");
                self.hide(other, rect_counter);
            }
            (Q::Internal { .. }, Q::Leaf { visible, .. }) => {
                let mut count = 0;
                self.hidden_children(&mut count);
                rect_counter.fetch_sub(count, Ordering::Relaxed);
                if !visible {
                    *self = Self::Leaf {
                        bounds: self.get_area(),
                        visible: false,
                    };
                }
                rect_counter.fetch_add(1, Ordering::Relaxed);
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
                tl_self.hide(tl_other, rect_counter.clone());
                tr_self.hide(tr_other, rect_counter.clone());
                bl_self.hide(bl_other, rect_counter.clone());
                br_self.hide(br_other, rect_counter.clone());
            }
        }
    }

    /// Remove fog of war represented by other from self
    pub fn show(&mut self, other: &Self, rect_counter: Arc<AtomicUsize>) {
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
                    rect_counter.fetch_sub(1, Ordering::Relaxed);
                }
                return;
            }
            (Q::Leaf { visible, .. }, Q::Internal { .. }) => {
                if *visible {
                    return;
                }
                self.to_internal()
                    .expect("expected self to be able to split");
                self.show(other, rect_counter);
            }
            (Q::Internal { .. }, Q::Leaf { visible, .. }) => {
                let mut count = 0;
                self.hidden_children(&mut count);
                rect_counter.fetch_sub(count, Ordering::Relaxed);
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
                tl_self.show(tl_other, rect_counter.clone());
                tr_self.show(tr_other, rect_counter.clone());
                bl_self.show(bl_other, rect_counter.clone());
                br_self.show(br_other, rect_counter.clone());
            }
        }
    }

    /// Creates bigger quadtree squares when possible, if all leaf nodes have the same visibility
    /// modifier
    pub fn clean(&mut self, rect_counter: Arc<AtomicUsize>) {
        match self {
            Self::Internal {
                topleft,
                topright,
                bottomleft,
                bottomright,
            } => {
                topleft.clean(rect_counter.clone());
                topright.clean(rect_counter.clone());
                bottomleft.clean(rect_counter.clone());
                bottomright.clean(rect_counter.clone());
                let n1 = match topleft.visible() {
                    Ok(n) => n,
                    Err(_) => return,
                };
                let n2 = match topright.visible() {
                    Ok(n) => n,
                    Err(_) => return,
                };
                let n3 = match bottomleft.visible() {
                    Ok(n) => n,
                    Err(_) => return,
                };
                let n4 = match bottomright.visible() {
                    Ok(n) => n,
                    Err(_) => return,
                };
                if n1 && n2 && n3 && n4 {
                    *self = Self::Leaf {
                        bounds: self.get_area(),
                        visible: true,
                    }
                }
                if !n1 && !n2 && !n3 && !n4 {
                    *self = Self::Leaf {
                        bounds: self.get_area(),
                        visible: false,
                    };
                    rect_counter.fetch_sub(3, Ordering::Relaxed);
                }
            }
            Self::Leaf { .. } => return,
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

    /// return whether self is visible or not if it is an internal node it returns an error
    fn visible(&self) -> Result<bool, RustVttError> {
        match self {
            QuadtreeNode::Leaf { visible, .. } => Ok(*visible),
            QuadtreeNode::Internal { .. } => Err(RustVttError::InvalidInput),
        }
    }

    /// Update the given count for the amount of hidden children, also counts the current node if
    /// hidden, so initial call should be with an internal node
    fn hidden_children(&self, count: &mut usize) {
        match self {
            QuadtreeNode::Leaf { visible, .. } => {
                if !visible {
                    *count += 1
                }
            }
            QuadtreeNode::Internal {
                topleft,
                topright,
                bottomleft,
                bottomright,
            } => {
                topleft.hidden_children(count);
                topright.hidden_children(count);
                bottomleft.hidden_children(count);
                bottomright.hidden_children(count);
            }
        }
    }
}

//! The FogOfWar is quadtree that efficiently stores information on which pixels in the image are
//! covered by fog of war. This struct is used in the VTT struct and should generally only be accessed
//! via the VTT struct.

use geo::LineString;
/// A quadtree representing fog of war: child1: top left, child2: top right, child3: bottom left,
/// child4: bottom right
pub struct FogOfWar {
    hidden: bool,
    child1: Option<Box<FogOfWar>>,
    child2: Option<Box<FogOfWar>>,
    child3: Option<Box<FogOfWar>>,
    child4: Option<Box<FogOfWar>>,
}

pub enum Operation {
    HIDE,
    SHOW,
}

impl FogOfWar {
    /// Set the entire fog of war hidden area to true (hide everything)
    pub fn hide_all(&mut self) -> &mut Self {
        self.hidden = true;
        self.child1 = None;
        self.child2 = None;
        self.child3 = None;
        self.child4 = None;
        return self;
    }

    /// Set the entire fog of war hidden area to false (reveal everything)
    pub fn show_all(&mut self) -> &mut Self {
        self.hidden = false;
        self.child1 = None;
        self.child2 = None;
        self.child3 = None;
        self.child4 = None;
        return self;
    }

    pub fn update(&mut self, operation: Operation, polygon: &LineString) {
        //
        todo!("Given a polygon of what is visible or not, this function should convert this into a quad tree");
    }
}

impl Default for FogOfWar {
    fn default() -> Self {
        Self {
            hidden: false,
            child1: None,
            child2: None,
            child3: None,
            child4: None,
        }
    }
}

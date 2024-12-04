//! The FogOfWar is quadtree that efficiently stores information on which pixels in the image are
//! covered by fog of war. This struct is used in the VTT struct and should generally only be accessed
//! via the VTT struct.
/// A quadtree representing fog of war.
pub struct FogOfWar {
    hidden: bool,
    child1: Option<Box<FogOfWar>>,
    child2: Option<Box<FogOfWar>>,
    child3: Option<Box<FogOfWar>>,
    child4: Option<Box<FogOfWar>>,
}

impl FogOfWar {
    /// Set the entire fog of war hidden area to true
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

    pub fn update(&mut self) {
        todo!("Given pixel data of what is visible or not, this function should convert this into a quad tree");
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

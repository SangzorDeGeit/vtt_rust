//! # VTT_Rust
//!
//! VTT_Rust is a crate to work with uvtt files used in tabletop applications.
//! ## Basic Usage
//!
//! You can open a .uvtt (or dd2vtt) map using the `open_vtt` function:
//! ```
//! use vtt_rust::open_vtt;
//! use vtt_rust::VTT;
//!
//! let mut vtt: VTT = open_vtt("tests/resources/example1.dd2vtt").unwrap();
//! ```
//! Generally working with this struct will go as follows (subject to change):
//! - Call some function to edit a property (e.g. `set_ambient_light(NightTime)`)
//! - Update the image using `update_image()`
//! - Save or get a pixelbuffer of the image using `save_image(path)` or `get_pixbuf()` to use the new
//! image.
//!
//! If you plan on changing more then one property before revealing the image it is better to edit
//! all these properties at once and then updating the image.

mod errors;
mod fog_of_war;
mod helper;
mod vtt;
use anyhow::Result;
use std::{fs::File, io::Read, path::Path};

pub use vtt::VTT;

/// Open a vtt file and store the contents in memory
pub fn open_vtt<P: AsRef<Path>>(path: P) -> Result<VTT> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let vtt: VTT = serde_json::from_str(&contents)?;
    return Ok(vtt);
}

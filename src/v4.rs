//! Native MathML v4 XML API backed by Moxml and Leptris.

use lutaml_rs::{contracts, xml as xml_rt};

#[path = "generated_xml/v4/models.rs"]
pub mod models;

#[path = "generated_xml/v4/xml_codecs.rs"]
pub mod xml_codecs;

pub use models::Mml_V4_Math as Math;
pub use xml_codecs::{read_Mml_V4_Math_xml as from_xml, write_Mml_V4_Math_xml as to_xml};

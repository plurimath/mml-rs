//! Native MathML v4 XML API backed by Moxml and Leptris.

use lutaml_rs::{contracts, xml as xml_rt};

#[path = "generated_xml/v4/models.rs"]
pub mod models;

#[path = "generated_xml/v4/xml_codecs.rs"]
pub mod xml_codecs;

pub use models::Mml_V4_Math as Math;

/// Reads MathML 4 the way Ruby `Mml.parse` does; an unnamespaced `<math>`
/// root is read as MathML.
pub fn from_xml(input: &[u8]) -> contracts::ContractResult<Math> {
    xml_codecs::read_Mml_V4_Math_xml(&crate::ruby_xml::reader_input(input))
}

/// Writes MathML 4 with the bytes Ruby `Mml` `to_xml` gives: default
/// namespace, two-space indentation.
pub fn to_xml(math: &Math) -> contracts::ContractResult<String> {
    let xml = xml_codecs::write_Mml_V4_Math_xml(math)?;
    crate::ruby_xml::ruby_layout("Mml::V4::Math@mml_v4", xml)
}

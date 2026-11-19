//! Native MathML v3 XML API backed by Moxml and Leptris.

use lutaml_rs::{contracts, xml as xml_rt};

#[path = "generated_xml/v3/models.rs"]
pub mod models;

#[path = "generated_xml/v3/xml_codecs.rs"]
pub mod xml_codecs;

pub use models::Mml_V3_Math as Math;

/// Reads MathML 3 the way Ruby `Mml.parse` does; an unnamespaced `<math>`
/// root is read as MathML.
pub fn from_xml(input: &[u8]) -> contracts::ContractResult<Math> {
    xml_codecs::read_Mml_V3_Math_xml(&crate::ruby_xml::reader_input(input))
}

/// Writes MathML 3 with the bytes Ruby `Mml` `to_xml` gives: default
/// namespace, two-space indentation.
pub fn to_xml(math: &Math) -> contracts::ContractResult<String> {
    let xml = xml_codecs::write_Mml_V3_Math_xml(math)?;
    crate::ruby_xml::ruby_layout("Mml::V3::Math@mml_v3", xml)
}

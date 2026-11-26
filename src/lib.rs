//! Typed MathML models generated from the Ruby MML source declarations.

pub const MODEL_MANIFEST: &[u8] = include_bytes!("../model/manifest.json");

#[cfg(any(feature = "v3", feature = "v4"))]
mod ruby_xml;

#[cfg(feature = "v3")]
pub mod v3;

#[cfg(feature = "v4")]
pub mod v4;

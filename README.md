# MathML models for Rust

This package provides generated MathML v3 and v4 models and native XML
codecs through the Lutaml XML runtime, Moxml, and Leptris. The original
Ruby model declaration bundles and their source identities are included.

The default `v3` feature exposes `mml_rs::v3::from_xml(&[u8])` and
`mml_rs::v3::to_xml(&Math)`. Enable `v4` for the corresponding v4 API.
Both return the underlying Lutaml contract result and preserve its typed
failures.

Build the versions independently with `cargo test --no-default-features
--features v3 --test native_xml` and the corresponding `v4` command.
The XML codecs use the complete source graph. Enable `model-serde`
when you also need Serde traits for the generated model structs.

The current source candidate is verified with the exact local
`lutaml-rs` runtime recorded in `model/codegen.json`, supplied through a
Cargo patch. Remote dependency resolution awaits that runtime revision.

To regenerate the native sources, apply the patch in
`model/xml-models-optional-serde.patch` to the framework revision pinned in
`model/codegen.json`. Build its `mml-xml-codegen` binary with the `xml`
feature. Pass each `model/mathml-vN.bundle.json` and the corresponding
`src/generated_xml/vN` directory to that binary. The provenance file
records the generated hashes for both versions.

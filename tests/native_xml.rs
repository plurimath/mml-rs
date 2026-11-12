use lutaml_rs::xml::moxml_binding::moxml::document::{ContentRow, Document, ElementRow};
use serde_json::{json, Value};

fn tree(rows: &[ElementRow], ordinal: usize) -> Value {
    let element = &rows[ordinal];
    let mut attributes: Vec<_> = element
        .attributes
        .iter()
        .map(|attr| {
            (
                attr.name.namespace.as_deref(),
                &attr.name.local,
                &attr.value,
            )
        })
        .collect();
    attributes.sort();
    let has_elements = element
        .content
        .iter()
        .any(|node| matches!(node, ContentRow::Element(_)));
    let children: Vec<_> = element
        .content
        .iter()
        .filter_map(|node| match node {
            ContentRow::Element(ordinal) => Some(tree(rows, *ordinal)),
            ContentRow::Text(text) if has_elements && text.trim().is_empty() => None,
            ContentRow::Text(text) => Some(json!(["text", text])),
            ContentRow::CData(text) => Some(json!(["cdata", text])),
            ContentRow::Comment(text) => Some(json!(["comment", text])),
            ContentRow::ProcessingInstruction { target, data } => {
                Some(json!(["instruction", target, data]))
            }
        })
        .collect();
    json!([
        element.name.namespace.as_deref(),
        element.name.local,
        attributes,
        children
    ])
}

fn xml_tree(xml: &str) -> Value {
    let doc = Document::parse(xml.as_bytes()).unwrap();
    tree(&doc.element_rows(), 0)
}

fn run(version: u64, replay: impl Fn(&str, &[u8]) -> lutaml_rs::ContractResult<String>) {
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/native-xml-cases.json")).unwrap();
    let cases = corpus["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["version"] == version);
    let mut count = 0;
    for row in cases {
        let id = row["id"].as_str().unwrap();
        let model = row["model"].as_str().unwrap();
        let input = row["input"].as_str().unwrap();
        let actual =
            replay(model, input.as_bytes()).unwrap_or_else(|error| panic!("{id}: {error}"));
        assert_eq!(
            xml_tree(&actual),
            xml_tree(row["expected_xml"].as_str().unwrap()),
            "{id}"
        );
        count += 1;
    }
    assert_eq!(count, 3);
    assert!(replay("Math", b"<math xmlns='urn:not-mathml'><mi>x</mi></math>").is_err());
    println!("MathML v{version}: {count} native trees match and foreign namespace refused");
}

#[cfg(feature = "v3")]
#[test]
fn native_mathml_v3_trees() {
    use mml_rs::v3::{from_xml, to_xml, xml_codecs};
    run(3, |model, input| match model {
        "Mi" => xml_codecs::read_Mml_V3_Mi_xml(input)
            .and_then(|value| xml_codecs::write_Mml_V3_Mi_xml(&value)),
        "Math" => from_xml(input).and_then(|value| to_xml(&value)),
        _ => panic!("unknown source fixture model: {model}"),
    });
}

#[cfg(feature = "v4")]
#[test]
fn native_mathml_v4_trees() {
    use mml_rs::v4::{from_xml, to_xml, xml_codecs};
    run(4, |model, input| match model {
        "Mi" => xml_codecs::read_Mml_V4_Mi_xml(input)
            .and_then(|value| xml_codecs::write_Mml_V4_Mi_xml(&value)),
        "Math" => from_xml(input).and_then(|value| to_xml(&value)),
        _ => panic!("unknown source fixture model: {model}"),
    });
}

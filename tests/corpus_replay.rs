//! Replays the model-roundtrip rows of a pinned plurimath-testsuite mathml
//! generation extracted by `.github/scripts/extract-corpus.py`. Set
//! `MML_CORPUS_CASES` to the extracted JSON; `MML_CORPUS_REPORT`
//! optionally receives every outcome.
//!
//! Each row's input goes through `mml_rs::v3::from_xml` and `to_xml` (the
//! MathML 3 models `Mml.parse` builds by default), and the written XML must equal the
//! row's expected `to_xml` output byte for byte. A row whose expected outcome
//! is a refusal must be refused. A failing output is classed as
//! `output-bytes` (same element tree, different bytes) or `output-tree`.
//! The expected Ruby model wire is not compared here: it names Ruby classes
//! and Lutaml attributes, which the generated Rust models do not expose.
//!
//! The ledger is append-only: a row whose id has an `-r2` sibling is
//! superseded by it and only the sibling runs, as in the Ruby gem's replay.
use lutaml_rs::xml::moxml_binding::moxml::document::{ContentRow, Document, ElementRow};
use mml_rs::v3::{from_xml, to_xml};
use serde_json::{json, Value};
use std::collections::HashSet;

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

fn char_floor(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn xml_tree(xml: &str) -> Option<Value> {
    let doc = Document::parse(xml.as_bytes()).ok()?;
    Some(tree(&doc.element_rows(), 0))
}

/// What `Mml.parse(input).to_xml` does with the default MathML 3 models.
fn roundtrip(input: &str) -> Result<String, String> {
    let model = from_xml(input.as_bytes()).map_err(|e| e.to_string())?;
    to_xml(&model).map_err(|e| e.to_string())
}

/// `None` when the row passes, otherwise `(kind, detail)`.
fn failure(row: &Value) -> Option<(&'static str, String)> {
    let input = row["input"]["str"].as_str().unwrap();
    assert!(
        row["options"].as_object().is_none_or(|o| o.is_empty()),
        "{}: unsupported options",
        row["id"]
    );
    let result = roundtrip(input);
    if row["expected"].get("refusal").is_some() {
        return result.ok().map(|xml| ("accepted", xml));
    }
    let expected = row["expected"]["output"]["str"].as_str().unwrap();
    match result {
        Err(error) => Some(("refused", error)),
        Ok(actual) if actual == expected => None,
        Ok(actual) => {
            let same_tree = xml_tree(&actual).is_some() && xml_tree(&actual) == xml_tree(expected);
            let at = actual
                .bytes()
                .zip(expected.bytes())
                .take_while(|(a, e)| a == e)
                .count();
            let from = char_floor(&actual, at.saturating_sub(80));
            let to = char_floor(&actual, (at + 160).min(actual.len()));
            let kind = if same_tree {
                "output-bytes"
            } else {
                "output-tree"
            };
            Some((
                kind,
                format!("differs at byte {at}: …{}", &actual[from..to]),
            ))
        }
    }
}

#[test]
#[ignore = "pinned generation replay runs in CI"]
fn pinned_generation_replay() {
    let path = std::env::var("MML_CORPUS_CASES").expect("MML_CORPUS_CASES");
    let corpus: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = corpus["cases"].as_array().unwrap();
    let ids: HashSet<&str> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
    let superseded: HashSet<String> = ids
        .iter()
        .filter(|id| id.contains("-r2"))
        .map(|id| id.replacen("-r2", "", 1))
        .collect();
    let mut outcomes = Vec::new();
    let mut failures = 0usize;
    for row in rows {
        assert_eq!(row["boundary"], "model-roundtrip", "{}", row["id"]);
        if superseded.contains(row["id"].as_str().unwrap()) {
            continue;
        }
        let failure = failure(row);
        failures += usize::from(failure.is_some());
        let (kind, detail) = match failure {
            Some((kind, detail)) => (
                Some(kind),
                Some(detail.chars().take(800).collect::<String>()),
            ),
            None => (None, None),
        };
        outcomes.push(
            json!({"id": row["id"], "payload": row["payload"], "kind": kind, "failure": detail}),
        );
    }
    assert_eq!(outcomes.len() + superseded.len(), rows.len());
    let mut kinds = std::collections::BTreeMap::<String, usize>::new();
    for outcome in &outcomes {
        if let Some(kind) = outcome["kind"].as_str() {
            *kinds.entry(kind.into()).or_default() += 1;
        }
    }
    let report = json!({
        "language": corpus["language"],
        "generation": corpus["generation"],
        "revision": corpus["revision"],
        "executed": outcomes.len(),
        "superseded": superseded.len(),
        "failed": failures,
        "kinds": kinds,
        "outcomes": outcomes,
    });
    if let Ok(out) = std::env::var("MML_CORPUS_REPORT") {
        std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    println!(
        "{} rows executed, {} superseded, {failures} failed {kinds:?}",
        outcomes.len(),
        superseded.len()
    );
    let shown: Vec<_> = outcomes
        .iter()
        .filter(|o| !o["kind"].is_null())
        .take(20)
        .collect();
    assert_eq!(
        failures,
        0,
        "{failures} of {} rows failed {kinds:?}: {shown:#?}",
        outcomes.len()
    );
}

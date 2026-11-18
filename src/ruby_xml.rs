//! The XML conventions of the Ruby `Mml` gem, layered over the generated
//! codecs so `from_xml` / `to_xml` give the same bytes as `Mml.parse(xml).to_xml`.
//!
//! - Input: a `<math>` root in no namespace is read as MathML, as Ruby does.
//!   Every unnamespaced element of such a document moves into the MathML
//!   namespace. Malformed XML is still refused by the strict parser.
//! - Output: MathML elements use the default namespace (`xmlns="…"` on the
//!   root, no prefix), element-only content is indented by two spaces, and an
//!   empty element whose model maps text content is written `<mi></mi>`.

use lutaml_rs::contracts::{ContractResult, ErrorReason};
use lutaml_rs::xml::moxml_binding::fail;
use lutaml_rs::xml::moxml_binding::moxml::document::{
    ContentRow, ContentSpec, Document, ElementRow, ElementSpec, QName, SerializeOptions,
};
use std::borrow::Cow;
use std::sync::Arc;

pub(crate) const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";

/// Elements whose models map text content (`map_content`): Ruby writes them
/// as a start and end tag even when the content is empty.
const TEXT_CONTENT: &[&str] = &[
    "annotation",
    "cbytes",
    "ci",
    "cn",
    "cs",
    "csymbol",
    "mi",
    "mn",
    "mo",
    "ms",
    "mtext",
];

fn rebuild(
    rows: &[ElementRow],
    ordinal: usize,
    name: &dyn Fn(&QName) -> QName,
    pair_empty: bool,
) -> ElementSpec {
    let row = &rows[ordinal];
    let mut content: Vec<ContentSpec> = row
        .content
        .iter()
        .map(|node| match node {
            ContentRow::Element(child) => {
                ContentSpec::Element(rebuild(rows, *child, name, pair_empty))
            }
            ContentRow::Text(text) => ContentSpec::Text(text.clone()),
            ContentRow::CData(text) => ContentSpec::CData(text.clone()),
            ContentRow::Comment(text) => ContentSpec::Comment(text.clone()),
            ContentRow::ProcessingInstruction { target, data } => {
                ContentSpec::ProcessingInstruction {
                    target: target.clone(),
                    data: data.clone(),
                }
            }
        })
        .collect();
    let element = name(&row.name);
    if pair_empty
        && content.is_empty()
        && element.namespace.as_deref() == Some(MATHML_NS)
        && TEXT_CONTENT.contains(&element.local.as_str())
    {
        content.push(ContentSpec::Text(String::new()));
    }
    ElementSpec {
        name: element,
        // The serializer declares every namespace a name needs; dropping the
        // source declarations leaves exactly one default MathML binding.
        namespaces: vec![],
        attributes: row.attributes.clone(),
        content,
    }
}

/// The input the generated reader sees: unchanged unless the root is an
/// unnamespaced `<math>`, which is moved into the MathML namespace.
pub(crate) fn reader_input(input: &[u8]) -> Cow<'_, [u8]> {
    let Ok(doc) = Document::parse(input) else {
        // Let the generated reader report the parse error itself.
        return Cow::Borrowed(input);
    };
    let rows = doc.element_rows();
    if rows[0].name.namespace.is_some() || rows[0].name.local != "math" {
        return Cow::Borrowed(input);
    }
    let mathml: Arc<str> = Arc::from(MATHML_NS);
    let name = |q: &QName| match q.namespace {
        Some(_) => q.clone(),
        None => QName {
            prefix: None,
            local: q.local.clone(),
            namespace: Some(mathml.clone()),
        },
    };
    let mut spec = rebuild(&rows, 0, &name, false);
    keep_prefixed_declarations(&rows, 0, &mut spec);
    match Document::build(spec).and_then(|doc| doc.to_xml()) {
        Ok(xml) => Cow::Owned(xml.into_bytes()),
        Err(_) => Cow::Borrowed(input),
    }
}

/// `rebuild` drops declarations; on input, prefixed ones may still be needed
/// by QName-valued attributes, so they stay on the elements that made them.
fn keep_prefixed_declarations(rows: &[ElementRow], ordinal: usize, spec: &mut ElementSpec) {
    spec.namespaces = rows[ordinal]
        .namespaces
        .iter()
        .filter(|decl| decl.prefix.is_some())
        .cloned()
        .collect();
    let children = rows[ordinal].content.iter().filter_map(|node| match node {
        ContentRow::Element(child) => Some(*child),
        _ => None,
    });
    let specs = spec.content.iter_mut().filter_map(|node| match node {
        ContentSpec::Element(child) => Some(child),
        _ => None,
    });
    for (child, child_spec) in children.zip(specs) {
        keep_prefixed_declarations(rows, child, child_spec);
    }
}

/// Rewrites the generated writer's compact, `mml:`-prefixed output in the
/// layout Ruby writes.
pub(crate) fn ruby_layout(id: &str, xml: String) -> ContractResult<String> {
    let bad = |e: lutaml_rs::xml::moxml_binding::moxml::document::Error| {
        fail(
            ErrorReason::NotSerializable,
            id,
            None,
            None,
            "/output",
            e.to_string(),
        )
    };
    let doc = Document::parse(xml.as_bytes()).map_err(bad)?;
    let rows = doc.element_rows();
    let name = |q: &QName| match q.namespace.as_deref() {
        Some(MATHML_NS) => QName {
            prefix: None,
            ..q.clone()
        },
        _ => q.clone(),
    };
    let options = SerializeOptions {
        xml_declaration: false,
        indent: Some(2),
    };
    Document::build(rebuild(&rows, 0, &name, true))
        .and_then(|doc| doc.to_xml_with(&options))
        .map_err(bad)
}

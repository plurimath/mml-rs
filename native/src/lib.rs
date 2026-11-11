//! One owned native model per handle, with mapped JSON and XML projections.
use serde_json::{json, Value};
use std::{ffi::{c_char, CString}, panic::{catch_unwind, AssertUnwindSafe}};

#[cfg(all(feature = "v3", feature = "v4"))]
compile_error!("Build one MathML version per native library.");

use lutaml_rs::contracts;
use snapshots::Snapshot;

#[cfg(feature = "v3")]
use mml_rs::v3::models;
#[cfg(feature = "v3")]
use mml_rs::v3::xml_codecs::{
    read_Mml_V3_Math_xml as read_math, write_Mml_V3_Math_xml as write_math,
    read_Mml_V3_Mi_xml as read_mi, write_Mml_V3_Mi_xml as write_mi,
};
#[cfg(feature = "v3")]
use models::{Mml_V3_Math as Math, Mml_V3_Mi as Mi};
#[cfg(feature = "v3")]
#[path = "generated/v3/snapshots.rs"]
mod snapshots;

#[cfg(feature = "v4")]
use mml_rs::v4::models;
#[cfg(feature = "v4")]
use mml_rs::v4::xml_codecs::{
    read_Mml_V4_Math_xml as read_math, write_Mml_V4_Math_xml as write_math,
    read_Mml_V4_Mi_xml as read_mi, write_Mml_V4_Mi_xml as write_mi,
};
#[cfg(feature = "v4")]
use models::{Mml_V4_Math as Math, Mml_V4_Mi as Mi};
#[cfg(feature = "v4")]
#[path = "generated/v4/snapshots.rs"]
mod snapshots;

enum Model { Math(Math), Mi(Mi) }
struct Handle { model: Model }

fn contract(error: Box<contracts::ContractError>) -> Value {
    serde_json::to_value(error).expect("contract errors have a JSON representation")
}

fn response(operation: impl FnOnce() -> Result<Value, Value>) -> *mut c_char {
    let result = match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(value)) => json!({"ok": true, "value": value}),
        Ok(Err(error)) => json!({"ok": false, "error": error}),
        Err(payload) => {
            let message = payload.downcast_ref::<String>().map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied()).unwrap_or("native operation panicked");
            json!({"ok": false, "error": {"category": "native_panic", "detail": message}})
        }
    };
    CString::new(result.to_string()).expect("JSON escapes NUL bytes").into_raw()
}

/// Return source declarations shared by the parsed native models.
#[no_mangle]
pub extern "C" fn mml_native_info() -> *mut c_char {
    response(|| Ok(json!({
        "version": if cfg!(feature = "v4") { 4 } else { 3 },
        "snapshot": "mml-typed-model-snapshot-v1",
        "scalar_encoding": "portable-exact-json-v1",
        "presence": "absent omitted; null and present empty values retained",
        "ordered_content": "lml-ordered-content-v1",
        "model_manifest": serde_json::from_slice::<Value>(mml_rs::MODEL_MANIFEST)
            .expect("embedded source manifest is JSON"),
    })))
}

/// Parse a UTF-8 XML buffer. Kind 0 selects Math, kind 1 selects Mi.
/// A successful result owns one handle that must be released exactly once.
#[no_mangle]
pub unsafe extern "C" fn mml_native_parse(data: *const u8, length: usize, kind: u32) -> *mut c_char {
    response(|| {
        if data.is_null() {
            return Err(json!({"category": "invalid_argument", "detail": "XML buffer is null"}));
        }
        let input = std::slice::from_raw_parts(data, length);
        let model = match kind {
            0 => Model::Math(read_math(input).map_err(contract)?),
            1 => Model::Mi(read_mi(input).map_err(contract)?),
            _ => return Err(json!({"category": "invalid_argument", "detail": "unknown root model"})),
        };
        Ok(json!({"handle": Box::into_raw(Box::new(Handle { model })) as usize}))
    })
}

/// Snapshot the native model's presence-aware fields and XML ordering metadata.
#[no_mangle]
pub unsafe extern "C" fn mml_native_snapshot(handle: *const Handle) -> *mut c_char {
    response(|| {
        let handle = handle.as_ref().ok_or_else(|| json!({"category": "closed_handle"}))?;
        match &handle.model {
            Model::Math(model) => model.snapshot(),
            Model::Mi(model) => model.snapshot(),
        }.map_err(|error| json!({"category": "snapshot_serialization", "detail": error.to_string()}))
    })
}

/// Serialize XML from the same owned typed model.
#[no_mangle]
pub unsafe extern "C" fn mml_native_xml(handle: *const Handle) -> *mut c_char {
    response(|| {
        let handle = handle.as_ref().ok_or_else(|| json!({"category": "closed_handle"}))?;
        let xml = match &handle.model {
            Model::Math(model) => write_math(model),
            Model::Mi(model) => write_mi(model),
        }.map_err(contract)?;
        Ok(Value::String(xml))
    })
}

/// The caller must release each successful parse handle once and cease using it.
#[no_mangle]
pub unsafe extern "C" fn mml_native_free(handle: *mut Handle) {
    if !handle.is_null() { drop(Box::from_raw(handle)); }
}

/// Release each returned response buffer once.
#[no_mangle]
pub unsafe extern "C" fn mml_native_string_free(buffer: *mut c_char) {
    if !buffer.is_null() { drop(CString::from_raw(buffer)); }
}

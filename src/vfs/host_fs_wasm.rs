//! WASM-side [`HostFs`] implementation backed by JavaScript callbacks.
//!
//! [`WasmHostFs`] holds `js_sys::Function` references (one per operation)
//! that the TypeScript side registers at mount time.  Each method invokes
//! the corresponding JS function synchronously; the JS side reads from a
//! pre-populated cache (for reads) or queues async writes for later flush.

use super::host_fs::{HostEntry, HostFs};
use js_sys::{Function, Reflect};
use wasm_bindgen::prelude::*;

/// WASM-side `HostFs` that delegates to JavaScript callback functions.
///
/// The TypeScript `HostFsManager` creates an object with one function per
/// operation and passes it to `register_host_fs()`.  This struct extracts
/// those functions and calls them synchronously when the VFS needs to
/// read or write host files.
pub struct WasmHostFs {
    list_dir_fn: Function,
    read_file_fn: Function,
    read_file_lines_fn: Function,
    file_line_count_fn: Function,
    write_file_fn: Function,
    append_file_fn: Function,
    mkdir_fn: Function,
    touch_fn: Function,
    rm_fn: Function,
    rm_recursive_fn: Function,
    file_size_fn: Function,
    exists_fn: Function,
    is_dir_fn: Function,
}

/// Helper to extract a `Function` from a JS object by key.
fn get_fn(obj: &JsValue, key: &str) -> Function {
    Reflect::get(obj, &JsValue::from_str(key))
        .unwrap_or_else(|_| panic!("callbacks.{key} missing"))
        .dyn_into::<Function>()
        .unwrap_or_else(|_| panic!("callbacks.{key} is not a function"))
}

/// Helper to call a JS function with a single string argument and return
/// the result as a `String`.
fn call1_str(fn_: &Function, arg: &str) -> Result<String, String> {
    let result = fn_
        .call1(&JsValue::NULL, &JsValue::from_str(arg))
        .map_err(|e| format!("JS error: {:?}", e))?;
    Ok(result.as_string().unwrap_or_default())
}

/// Helper to call a JS function with two string arguments.
fn call2_str(fn_: &Function, a: &str, b: &str) -> Result<String, String> {
    let result = fn_
        .call2(&JsValue::NULL, &JsValue::from_str(a), &JsValue::from_str(b))
        .map_err(|e| format!("JS error: {:?}", e))?;
    Ok(result.as_string().unwrap_or_default())
}

impl WasmHostFs {
    /// Create a new `WasmHostFs` from a JS callbacks object.
    ///
    /// The object must have functions for: `list_dir`, `read_file`,
    /// `read_file_lines`, `file_line_count`, `write_file`, `append_file`,
    /// `mkdir`, `touch`, `rm`, `rm_recursive`, `file_size`, `exists`, `is_dir`.
    pub fn new(callbacks: &JsValue) -> Self {
        WasmHostFs {
            list_dir_fn: get_fn(callbacks, "list_dir"),
            read_file_fn: get_fn(callbacks, "read_file"),
            read_file_lines_fn: get_fn(callbacks, "read_file_lines"),
            file_line_count_fn: get_fn(callbacks, "file_line_count"),
            write_file_fn: get_fn(callbacks, "write_file"),
            append_file_fn: get_fn(callbacks, "append_file"),
            mkdir_fn: get_fn(callbacks, "mkdir"),
            touch_fn: get_fn(callbacks, "touch"),
            rm_fn: get_fn(callbacks, "rm"),
            rm_recursive_fn: get_fn(callbacks, "rm_recursive"),
            file_size_fn: get_fn(callbacks, "file_size"),
            exists_fn: get_fn(callbacks, "exists"),
            is_dir_fn: get_fn(callbacks, "is_dir"),
        }
    }
}

impl HostFs for WasmHostFs {
    fn list_dir(&self, host_path: &str) -> Result<Vec<HostEntry>, String> {
        let json = call1_str(&self.list_dir_fn, host_path)?;
        serde_json::from_str(&json).map_err(|e| format!("list_dir parse error: {}", e))
    }

    fn read_file(&self, host_path: &str) -> Result<String, String> {
        call1_str(&self.read_file_fn, host_path)
    }

    fn read_file_lines(
        &self,
        host_path: &str,
        start: usize,
        count: usize,
    ) -> Result<String, String> {
        let result = self
            .read_file_lines_fn
            .call3(
                &JsValue::NULL,
                &JsValue::from_str(host_path),
                &JsValue::from_f64(start as f64),
                &JsValue::from_f64(count as f64),
            )
            .map_err(|e| format!("JS error: {:?}", e))?;
        Ok(result.as_string().unwrap_or_default())
    }

    fn file_line_count(&self, host_path: &str) -> Result<usize, String> {
        let result = call1_str(&self.file_line_count_fn, host_path)?;
        result
            .parse::<usize>()
            .map_err(|e| format!("file_line_count parse error: {}", e))
    }

    fn write_file(&self, host_path: &str, content: &str) -> Result<String, String> {
        call2_str(&self.write_file_fn, host_path, content)
    }

    fn append_file(&self, host_path: &str, content: &str) -> Result<String, String> {
        call2_str(&self.append_file_fn, host_path, content)
    }

    fn mkdir(&self, host_path: &str) -> Result<String, String> {
        call1_str(&self.mkdir_fn, host_path)
    }

    fn touch(&self, host_path: &str) -> Result<String, String> {
        call1_str(&self.touch_fn, host_path)
    }

    fn rm(&self, host_path: &str) -> Result<String, String> {
        call1_str(&self.rm_fn, host_path)
    }

    fn rm_recursive(&self, host_path: &str) -> Result<String, String> {
        call1_str(&self.rm_recursive_fn, host_path)
    }

    fn file_size(&self, host_path: &str) -> Result<usize, String> {
        let result = call1_str(&self.file_size_fn, host_path)?;
        result
            .parse::<usize>()
            .map_err(|e| format!("file_size parse error: {}", e))
    }

    fn exists(&self, host_path: &str) -> Result<bool, String> {
        let result = call1_str(&self.exists_fn, host_path)?;
        match result.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!(
                "exists parse error: expected 'true'/'false', got '{}'",
                result
            )),
        }
    }

    fn is_dir(&self, host_path: &str) -> Result<bool, String> {
        let result = call1_str(&self.is_dir_fn, host_path)?;
        match result.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!(
                "is_dir parse error: expected 'true'/'false', got '{}'",
                result
            )),
        }
    }
}

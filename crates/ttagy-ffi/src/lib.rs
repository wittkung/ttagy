//! TTAgy C-ABI FFI 导出层 (Rust 2021 / Strict Defensive FFI Safety)

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;
use ttagy_client::{ClientConfig, TtagyClient, TtagyRequest};

pub const TTAGY_OK: i32 = 0;
pub const TTAGY_ERR_INVALID_ARGUMENT: i32 = -1;
pub const TTAGY_ERR_NULL_POINTER: i32 = -2;
pub const TTAGY_ERR_INIT_FAILED: i32 = -3;
pub const TTAGY_ERR_REQUEST_FAILED: i32 = -4;
pub const TTAGY_ERR_SERIALIZATION: i32 = -5;
pub const TTAGY_ERR_PANIC: i32 = -999;

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("").unwrap());
}

fn set_last_error(msg: impl Into<String>) {
    let err_str = msg.into();
    LAST_ERROR.with(|cell| {
        if let Ok(c_str) = CString::new(err_str) {
            *cell.borrow_mut() = c_str;
        }
    });
}

pub struct TtagyClientContext {
    pub client: Arc<TtagyClient>,
    pub runtime: Runtime,
}

#[repr(C)]
pub struct ttagy_client_t {
    _opaque: [u8; 0],
}

macro_rules! ffi_guard {
    ($err_code:expr, $body:expr) => {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(res) => res,
            Err(panic_err) => {
                let msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_err.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown Rust FFI panic".to_string()
                };
                set_last_error(format!("Rust Panic caught: {}", msg));
                $err_code
            }
        }
    };
}

/// 创建 TTAGY 客户端实例
#[no_mangle]
pub unsafe extern "C" fn ttagy_client_create(
    config_json: *const c_char,
    out_client: *mut *mut ttagy_client_t,
) -> i32 {
    ffi_guard!(TTAGY_ERR_PANIC, {
        if out_client.is_null() {
            set_last_error("out_client pointer is NULL");
            return TTAGY_ERR_NULL_POINTER;
        }
        *out_client = ptr::null_mut();

        let config = if !config_json.is_null() {
            let c_str = CStr::from_ptr(config_json);
            let json_str = match c_str.to_str() {
                Ok(s) => s,
                Err(e) => {
                    set_last_error(format!("config_json is not valid UTF-8: {}", e));
                    return TTAGY_ERR_INVALID_ARGUMENT;
                }
            };
            match serde_json::from_str::<ClientConfig>(json_str) {
                Ok(cfg) => cfg,
                Err(e) => {
                    set_last_error(format!("Failed to parse ClientConfig JSON: {}", e));
                    return TTAGY_ERR_SERIALIZATION;
                }
            }
        } else {
            ClientConfig::default()
        };

        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                set_last_error(format!("Failed to build Tokio runtime: {}", e));
                return TTAGY_ERR_INIT_FAILED;
            }
        };

        let client = Arc::new(TtagyClient::new(config));
        let ctx = Box::new(TtagyClientContext { client, runtime });

        *out_client = Box::into_raw(ctx) as *mut ttagy_client_t;
        TTAGY_OK
    })
}

/// 释放 TTAGY 客户端实例
#[no_mangle]
pub unsafe extern "C" fn ttagy_client_free(client: *mut ttagy_client_t) {
    if client.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let ctx = Box::from_raw(client as *mut TtagyClientContext);
        drop(ctx);
    }));
}

/// 同步阻塞执行单轮推导
#[no_mangle]
pub unsafe extern "C" fn ttagy_client_chat(
    client: *mut ttagy_client_t,
    request_json: *const c_char,
    out_response_json: *mut *mut c_char,
) -> i32 {
    ffi_guard!(TTAGY_ERR_PANIC, {
        if client.is_null() {
            set_last_error("client handle is NULL");
            return TTAGY_ERR_NULL_POINTER;
        }
        if request_json.is_null() || out_response_json.is_null() {
            set_last_error("request_json or out_response_json is NULL");
            return TTAGY_ERR_NULL_POINTER;
        }
        *out_response_json = ptr::null_mut();

        let ctx = &*(client as *const TtagyClientContext);

        let c_str = CStr::from_ptr(request_json);
        let json_str = match c_str.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("request_json is not valid UTF-8: {}", e));
                return TTAGY_ERR_INVALID_ARGUMENT;
            }
        };

        let req = match serde_json::from_str::<TtagyRequest>(json_str) {
            Ok(r) => r,
            Err(e) => {
                set_last_error(format!("Failed to parse TtagyRequest JSON: {}", e));
                return TTAGY_ERR_SERIALIZATION;
            }
        };

        let client_arc = Arc::clone(&ctx.client);
        let resp_res = ctx.runtime.block_on(async move {
            client_arc.chat(req).await
        });

        match resp_res {
            Ok(resp) => {
                let out_str = match serde_json::to_string(&resp) {
                    Ok(s) => s,
                    Err(e) => {
                        set_last_error(format!("Failed to serialize TtagyResponse: {}", e));
                        return TTAGY_ERR_SERIALIZATION;
                    }
                };
                let c_resp = match CString::new(out_str) {
                    Ok(c) => c,
                    Err(e) => {
                        set_last_error(format!("CString creation failed (null byte in output): {}", e));
                        return TTAGY_ERR_SERIALIZATION;
                    }
                };
                *out_response_json = c_resp.into_raw();
                TTAGY_OK
            }
            Err(err_msg) => {
                set_last_error(format!("Client chat execution failed: {}", err_msg));
                TTAGY_ERR_REQUEST_FAILED
            }
        }
    })
}

/// 释放由 Rust 分配导出的 UTF-8 C 字符串
#[no_mangle]
pub unsafe extern "C" fn ttagy_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(CString::from_raw(ptr));
    }));
}

/// 获取当前调用线程最后一次发生的错误描述信息 (只读，调用方无需释放)
#[no_mangle]
pub unsafe extern "C" fn ttagy_last_error_message() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow().as_ptr()
    })
}

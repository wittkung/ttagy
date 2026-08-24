//! C-ABI FFI 集成与内存安全测试

use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr;
use ttagy::*;

fn get_mock_agy_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target");
    path.push("debug");
    path.push("mock-agy");
    if !path.exists() {
        panic!("mock-agy binary not found at {}", path.display());
    }
    path
}

#[test]
fn test_c_ffi_client_lifecycle_and_chat() {
    let mock_path = get_mock_agy_path();
    std::env::set_var("AGY_PATH", mock_path.to_str().unwrap());

    unsafe {
        let mut client: *mut ttagy_client_t = ptr::null_mut();
        let rc = ttagy_client_create(ptr::null(), &mut client);
        assert_eq!(rc, TTAGY_OK);
        assert!(!client.is_null());

        let req_json = CString::new(r#"{"prompt":"scenario:stream_normal","model":"gemini-3.7-flash"}"#).unwrap();
        let mut out_resp: *mut std::ffi::c_char = ptr::null_mut();

        let chat_rc = ttagy_client_chat(client, req_json.as_ptr(), &mut out_resp);
        assert_eq!(chat_rc, TTAGY_OK);
        assert!(!out_resp.is_null());

        let resp_str = CStr::from_ptr(out_resp).to_str().unwrap();
        assert!(resp_str.contains("Antigravity AI 助手"));

        // 内存释放验证
        ttagy_string_free(out_resp);
        ttagy_client_free(client);
    }
}

#[test]
fn test_c_ffi_defensive_null_pointers() {
    unsafe {
        let rc = ttagy_client_create(ptr::null(), ptr::null_mut());
        assert_eq!(rc, TTAGY_ERR_NULL_POINTER);

        let err_msg = CStr::from_ptr(ttagy_last_error_message()).to_str().unwrap();
        assert!(err_msg.contains("NULL"));

        let chat_rc = ttagy_client_chat(ptr::null_mut(), ptr::null(), ptr::null_mut());
        assert_eq!(chat_rc, TTAGY_ERR_NULL_POINTER);
    }
}

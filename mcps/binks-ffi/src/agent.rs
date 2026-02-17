//! FFI functions for Agent operations
//!
//! Provides C-compatible functions for creating and using the Agent.

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use binks_agent::{Agent, AgentBuilder};
use sysinfo_mcp::SysInfoMcpServer;

/// Opaque handle to a Binks Agent
pub struct BinksAgent {
    inner: Agent,
    runtime: tokio::runtime::Runtime,
}

/// Create a new Binks Agent with default embedded MCPs
///
/// Returns NULL on failure.
///
/// # Safety
///
/// The returned pointer must be freed with `binks_agent_free`.
#[no_mangle]
pub unsafe extern "C" fn binks_agent_new() -> *mut BinksAgent {
    binks_agent_new_with_model(ptr::null())
}

/// Create a new Binks Agent with a specified model
///
/// If model is NULL, uses "qwen2.5:7b" as default.
/// Returns NULL on failure.
///
/// # Safety
///
/// The returned pointer must be freed with `binks_agent_free`.
#[no_mangle]
pub unsafe extern "C" fn binks_agent_new_with_model(model: *const c_char) -> *mut BinksAgent {
    // Create tokio runtime for async operations
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return ptr::null_mut(),
    };

    // Parse model name or use default
    let model_str = if model.is_null() {
        "qwen2.5:7b".to_string()
    } else {
        match unsafe { CStr::from_ptr(model).to_str() } {
            Ok(s) => s.to_string(),
            Err(_) => return ptr::null_mut(),
        }
    };

    // Build agent with embedded MCP
    let agent_result = runtime.block_on(async {
        AgentBuilder::new()
            .with_model(&model_str)
            .with_embedded_mcp(SysInfoMcpServer::new())
            .build()
            .await
    });

    match agent_result {
        Ok(agent) => Box::into_raw(Box::new(BinksAgent {
            inner: agent,
            runtime,
        })),
        Err(_) => ptr::null_mut(),
    }
}

/// Send a message to the agent and get a response
///
/// Returns a C string that must be freed with `binks_string_free`.
/// Returns NULL on error.
///
/// # Safety
///
/// - `agent` must be a valid pointer from `binks_agent_new`
/// - `message` must be a valid null-terminated C string
/// - The returned string must be freed with `binks_string_free`
#[no_mangle]
pub unsafe extern "C" fn binks_agent_chat(
    agent: *mut BinksAgent,
    message: *const c_char,
) -> *mut c_char {
    if agent.is_null() || message.is_null() {
        return ptr::null_mut();
    }

    let agent = unsafe { &mut *agent };
    let message = match unsafe { CStr::from_ptr(message).to_str() } {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let response = agent.runtime.block_on(agent.inner.chat(message));

    match response {
        Ok(text) => match CString::new(text) {
            Ok(cs) => cs.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        Err(e) => {
            // Return error message as string
            match CString::new(format!("Error: {}", e)) {
                Ok(cs) => cs.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
    }
}

/// Get the last error message (if any)
///
/// Currently returns NULL. Future versions may implement error tracking.
#[no_mangle]
pub extern "C" fn binks_get_last_error() -> *const c_char {
    ptr::null()
}

/// Free a Binks Agent
///
/// # Safety
///
/// `agent` must be a valid pointer from `binks_agent_new` or NULL.
#[no_mangle]
pub unsafe extern "C" fn binks_agent_free(agent: *mut BinksAgent) {
    if !agent.is_null() {
        drop(Box::from_raw(agent));
    }
}

/// Free a string returned by binks functions
///
/// # Safety
///
/// `s` must be a valid pointer from a binks function or NULL.
#[no_mangle]
pub unsafe extern "C" fn binks_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Get the version of the binks-ffi library
///
/// Returns a static string that should not be freed.
#[no_mangle]
pub extern "C" fn binks_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = binks_version();
        assert!(!version.is_null());
        let version_str = unsafe { CStr::from_ptr(version).to_str().unwrap() };
        assert_eq!(version_str, "0.1.0");
    }

    #[test]
    fn test_null_safety() {
        // These should not crash - unsafe calls are safe with null pointers
        unsafe {
            binks_agent_free(ptr::null_mut());
            binks_string_free(ptr::null_mut());
            let result = binks_agent_chat(ptr::null_mut(), ptr::null());
            assert!(result.is_null());
        }
    }
}

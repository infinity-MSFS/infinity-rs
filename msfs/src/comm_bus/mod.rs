use crate::sys::*;
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    os::raw::{c_char, c_void},
    ptr::NonNull,
};

#[derive(Debug)]
pub enum CommBusError {
    NulByte(std::ffi::NulError),
    RegistrationFailed,
    CallFailed,
    NotRegistered,
}

impl std::fmt::Display for CommBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NulByte(e) => write!(f, "nul byte in string: {e}"),
            Self::RegistrationFailed => write!(f, "fsCommBusRegister returned false"),
            Self::CallFailed => write!(f, "fsCommBusCall returned false"),
            Self::NotRegistered => write!(f, "not registered"),
        }
    }
}

impl From<std::ffi::NulError> for CommBusError {
    fn from(e: std::ffi::NulError) -> Self {
        Self::NulByte(e)
    }
}

bitflags::bitflags! {
        // #[derive(Debug, Copy, Clone)]
    pub struct BroadcastFlags: u8 {
        const JS           = FsCommBusBroadcastFlags_FsCommBusBroadcast_JS as u8;
        const WASM         = FsCommBusBroadcastFlags_FsCommBusBroadcast_Wasm as u8;
        const WASM_SELF    = FsCommBusBroadcastFlags_FsCommBusBroadcast_WasmSelfCall as u8;

        const DEFAULT      = FsCommBusBroadcastFlags_FsCommBusBroadcast_Default as u8;
        const ALL_WASM     = FsCommBusBroadcastFlags_FsCommBusBroadcast_AllWasm as u8;
        const ALL          = FsCommBusBroadcastFlags_FsCommBusBroadcast_All as u8;
    }
}

impl BroadcastFlags {
    #[inline]
    fn to_ffi(self) -> FsCommBusBroadcastFlags {
        self.bits() as FsCommBusBroadcastFlags
    }
}

struct CallbackState {
    cb: Box<dyn FnMut(&[u8]) + 'static>,
}

extern "C" fn commbus_trampoline(buf: *const c_char, buf_size: u32, ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    let st = unsafe { &mut *(ctx as *mut CallbackState) };

    if buf.is_null() || buf_size == 0 {
        (st.cb)(&[]);
        return;
    }

    let bytes = unsafe { std::slice::from_raw_parts(buf as *const u8, buf_size as usize) };
    (st.cb)(bytes);
}

pub struct Subscription {
    event: CString,
    state: NonNull<CallbackState>,
}

impl Subscription {
    pub fn subscribe(event: &str, cb: impl FnMut(&[u8]) + 'static) -> Result<Self, CommBusError> {
        let event = CString::new(event)?;
        let st = Box::new(CallbackState { cb: Box::new(cb) });
        let state_ptr = NonNull::new(Box::into_raw(st)).expect("Box::into_raw never null");

        let ok = unsafe {
            fsCommBusRegister(
                event.as_ptr(),
                Some(commbus_trampoline),
                state_ptr.as_ptr() as *mut c_void,
            )
        };

        if !ok {
            unsafe {
                drop(Box::from_raw(state_ptr.as_ptr()));
            }
            return Err(CommBusError::RegistrationFailed);
        }

        Ok(Self {
            event,
            state: state_ptr,
        })
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        unsafe {
            let _ = fsCommBusUnregisterOneEvent(
                self.event.as_ptr(),
                Some(commbus_trampoline),
                self.state.as_ptr() as *mut c_void,
            );

            drop(Box::from_raw(self.state.as_ptr()));
        }
    }
}

pub fn call(event: &str, payload: &[u8], broadcast: BroadcastFlags) -> Result<(), CommBusError> {
    let event = CString::new(event)?;
    let ok = unsafe {
        fsCommBusCall(
            event.as_ptr(),
            payload.as_ptr() as *const c_char,
            payload.len() as u32,
            broadcast.to_ffi(),
        )
    };
    if ok {
        Ok(())
    } else {
        Err(CommBusError::CallFailed)
    }
}

#[derive(Deserialize)]
struct IncomingEnvelope {
    #[serde(rename = "requestId")]
    request_id: String,
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct OutgoingEnvelope<'a> {
    #[serde(rename = "requestId")]
    request_id: &'a str,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

pub fn reply_json(
    response_event: &str,
    request_id: &str,
    result: Result<serde_json::Value, &str>,
) -> Result<(), CommBusError> {
    let envelope = match result {
        Ok(val) => OutgoingEnvelope {
            request_id,
            ok: true,
            response: Some(val),
            error: None,
        },
        Err(e) => OutgoingEnvelope {
            request_id,
            ok: false,
            response: None,
            error: Some(e),
        },
    };

    let json_str = serde_json::to_string(&envelope)
        .unwrap_or_else(|_| r#"{"ok":false,"error":"SERIALIZE_FAILED"}"#.into());
    let mut bytes = json_str.into_bytes();
    bytes.push(0);

    call(response_event, &bytes, BroadcastFlags::JS)
}

pub struct JsonBridge {
    _sub: Subscription,
}

impl JsonBridge {
    pub fn register(
        request_event: &str,
        response_event: &str,
        mut handler: impl FnMut(serde_json::Value) -> Result<serde_json::Value, String> + 'static,
    ) -> Result<Self, CommBusError> {
        let resp_event = response_event.to_owned();

        let sub = Subscription::subscribe(request_event, move |bytes| {
            // Strip trailing nul bytes (C++ sends size+1)
            let trimmed = match bytes.iter().position(|&b| b == 0) {
                Some(pos) => &bytes[..pos],
                None => bytes,
            };

            let s = match std::str::from_utf8(trimmed) {
                Ok(s) => s,
                Err(_) => {
                    let _ = reply_json(&resp_event, "unknown", Err("INVALID_UTF8"));
                    return;
                }
            };

            if s.is_empty() {
                let _ = reply_json(&resp_event, "unknown", Err("EMPTY_PAYLOAD"));
                return;
            }

            let envelope: IncomingEnvelope = match serde_json::from_str::<IncomingEnvelope>(s) {
                Ok(e) if !e.request_id.is_empty() => e,
                Ok(_) => {
                    let _ = reply_json(&resp_event, "unknown", Err("MISSING_REQUEST_ID"));
                    return;
                }
                Err(_) => {
                    let _ = reply_json(&resp_event, "unknown", Err("REQUEST_PARSE_FAILED"));
                    return;
                }
            };

            let rid = &envelope.request_id;
            match handler(envelope.payload) {
                Ok(val) => {
                    let _ = reply_json(&resp_event, rid, Ok(val));
                }
                Err(e) => {
                    let _ = reply_json(&resp_event, rid, Err(&e));
                }
            }
        })?;

        Ok(Self { _sub: sub })
    }
}

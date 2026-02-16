use crate::sys::*;
use std::{
    ffi::CString,
    os::raw::{c_char, c_void},
    ptr::NonNull,
};

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
    pub fn subscribe(
        event: &str,
        cb: impl FnMut(&[u8]) + 'static,
    ) -> Result<Self, std::ffi::NulError> {
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
            // Surface a real error value without changing the public API.
            // Registration failure isn't a NUL error, but the old signature
            // uses `NulError`, so we synthesize one.
            return Err(CString::new(vec![0u8]).unwrap_err());
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

pub fn call(
    event: &str,
    payload: &[u8],
    broadcast: BroadcastFlags,
) -> Result<bool, std::ffi::NulError> {
    let event = CString::new(event)?;
    let ok = unsafe {
        fsCommBusCall(
            event.as_ptr(),
            payload.as_ptr() as *const c_char,
            payload.len() as u32,
            broadcast.to_ffi(),
        )
    };
    Ok(ok)
}

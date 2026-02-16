use crate::sys::*;
use std::{
    collections::HashMap,
    ffi::CString,
    os::raw::{c_char, c_void},
    sync::{LazyLock, Mutex}, // might not work in wasm, but we'll see
};

#[derive(Debug)]
pub enum NetError {
    Nul(std::ffi::NulError),
    Msfs(i32),
}

impl From<std::ffi::NulError> for NetError {
    fn from(value: std::ffi::NulError) -> Self {
        NetError::Nul(value)
    }
}

pub type NetResult<T> = Result<T, NetError>;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub request_id: FsNetworkRequestId,
    pub error_code: i32,
    pub data: Vec<u8>,
}

type Handler = Box<dyn FnOnce(HttpResponse) + Send + 'static>;

static HANDLERS: LazyLock<Mutex<HashMap<FsNetworkRequestId, Handler>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

extern "C" fn http_trampoline(
    request_id: FsNetworkRequestId,
    error_code: i32,
    _user_data: *mut c_void,
) {
    let data = unsafe {
        let ptr = fsNetworkHttpRequestGetData(request_id);
        let len = fsNetworkHttpRequestGetDataSize(request_id) as usize;
        if ptr.is_null() || len == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(ptr as *const u8, len).to_vec()
        }
    };

    let resp = HttpResponse {
        request_id,
        error_code,
        data,
    };

    drop_params(request_id);

    let handler = HANDLERS
        .lock()
        .ok()
        .and_then(|mut map| map.remove(&request_id));
    if let Some(h) = handler {
        h(resp);
    }
}

#[derive(Default)]
pub struct HttpParams {
    pub headers: Vec<String>,
    pub post_field: Option<String>,
    pub body: Vec<u8>,
}

impl HttpParams {
    fn into_ffi(self) -> NetResult<OwnedFfiParams> {
        OwnedFfiParams::new(self)
    }
}

struct OwnedFfiParams {
    _post_field: Option<CString>,
    _headers: Vec<CString>,
    header_ptrs: Vec<*mut c_char>,
    body: Vec<u8>,
    ffi: FsNetworkHttpRequestParam,
}

// This struct is only used to keep owned allocations alive until the MSFS
// networking request completes. The raw pointers inside `ffi` and `header_ptrs`
// point into memory owned by this struct and are not mutated after construction.
//
// MSFS may complete requests from another thread; storing these in a global
// mutex requires `Send`.
unsafe impl Send for OwnedFfiParams {}

impl OwnedFfiParams {
    fn new(p: HttpParams) -> NetResult<Self> {
        let post = match p.post_field {
            Some(s) => Some(CString::new(s)?),
            None => None,
        };

        let mut headers_cs = Vec::with_capacity(p.headers.len());
        for h in p.headers {
            headers_cs.push(CString::new(h)?);
        }

        let mut headers_ptrs: Vec<*mut c_char> = headers_cs
            .iter()
            .map(|c| c.as_ptr() as *mut c_char)
            .collect();

        let ffi = FsNetworkHttpRequestParam {
            postField: post
                .as_ref()
                .map(|c| c.as_ptr() as *mut c_char)
                .unwrap_or(std::ptr::null_mut()),
            headerOptions: if headers_ptrs.is_empty() {
                std::ptr::null_mut()
            } else {
                headers_ptrs.as_mut_ptr()
            },
            headerOptionsSize: headers_ptrs.len() as u32,
            data: if p.body.is_empty() {
                std::ptr::null_mut()
            } else {
                p.body.as_ptr() as *mut u8
            },
            dataSize: p.body.len() as u32,
        };

        Ok(Self {
            _post_field: post,
            _headers: headers_cs,
            header_ptrs: headers_ptrs,
            body: p.body,
            ffi,
        })
    }

    fn as_mut_ptr(&mut self) -> *mut FsNetworkHttpRequestParam {
        &mut self.ffi as *mut _
    }
}

pub enum Method {
    Get,
    Post,
    Put,
}

pub fn http_request(
    method: Method,
    url: &str,
    params: HttpParams,
    on_done: impl FnOnce(HttpResponse) + Send + 'static,
) -> NetResult<FsNetworkRequestId> {
    let url_c = CString::new(url)?;
    let mut owned = params.into_ffi()?;

    let id = unsafe {
        match method {
            Method::Get => fsNetworkHttpRequestGet(
                url_c.as_ptr(),
                owned.as_mut_ptr(),
                Some(http_trampoline),
                std::ptr::null_mut(),
            ),
            Method::Post => fsNetworkHttpRequestPost(
                url_c.as_ptr(),
                owned.as_mut_ptr(),
                Some(http_trampoline),
                std::ptr::null_mut(),
            ),
            Method::Put => fsNetworkHttpRequestPut(
                url_c.as_ptr(),
                owned.as_mut_ptr(),
                Some(http_trampoline),
                std::ptr::null_mut(),
            ),
        }
    };

    keep_params_alive(id, owned);

    HANDLERS.lock().unwrap().insert(id, Box::new(on_done));

    Ok(id)
}

static PARAMS: LazyLock<Mutex<HashMap<FsNetworkRequestId, OwnedFfiParams>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn keep_params_alive(id: FsNetworkRequestId, params: OwnedFfiParams) {
    PARAMS.lock().unwrap().insert(id, params);
}

fn drop_params(id: FsNetworkRequestId) {
    let _ = PARAMS.lock().ok().and_then(|mut m| m.remove(&id));
}

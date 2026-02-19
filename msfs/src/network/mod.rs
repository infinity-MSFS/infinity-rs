use crate::sys::*;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::CString,
    os::raw::{c_char, c_void},
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

type Handler = Box<dyn FnOnce(HttpResponse) + 'static>;

thread_local! {
    static HANDLERS: RefCell<HashMap<FsNetworkRequestId, Handler>> =
        RefCell::new(HashMap::new());

    static PARAMS: RefCell<HashMap<FsNetworkRequestId, OwnedFfiParams>> =
        RefCell::new(HashMap::new());
}

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

    let handler = HANDLERS.with(|m| m.borrow_mut().remove(&request_id));
    if let Some(h) = handler {
        h(resp);
    }
}

struct OwnedFfiParams {
    url: CString,
    _post_field: Option<CString>,
    _headers: Vec<CString>,
    _header_ptrs: Vec<*mut c_char>,
    _body: Vec<u8>,
    ffi: FsNetworkHttpRequestParam,
}

unsafe impl Send for OwnedFfiParams {}

impl OwnedFfiParams {
    fn new(url: &str, p: HttpParams) -> NetResult<Self> {
        let url_c = CString::new(url)?;

        let post = match p.post_field {
            Some(s) => Some(CString::new(s)?),
            None => None,
        };

        let mut headers_cs: Vec<CString> = p
            .headers
            .into_iter()
            .map(CString::new)
            .collect::<Result<_, _>>()?;

        let mut header_ptrs: Vec<*mut c_char> = headers_cs
            .iter()
            .map(|c| c.as_ptr() as *mut c_char)
            .collect();

        let ffi = FsNetworkHttpRequestParam {
            postField: post
                .as_ref()
                .map(|c| c.as_ptr() as *mut c_char)
                .unwrap_or(std::ptr::null_mut()),
            headerOptions: if header_ptrs.is_empty() {
                std::ptr::null_mut()
            } else {
                header_ptrs.as_mut_ptr()
            },
            headerOptionsSize: header_ptrs.len() as u32,
            data: if p.body.is_empty() {
                std::ptr::null_mut()
            } else {
                p.body.as_ptr() as *mut u8
            },
            dataSize: p.body.len() as u32,
        };

        Ok(Self {
            url: url_c,
            _post_field: post,
            _headers: headers_cs,
            _header_ptrs: header_ptrs,
            _body: p.body,
            ffi,
        })
    }

    fn url_ptr(&self) -> *const c_char {
        self.url.as_ptr()
    }

    fn ffi_ptr(&mut self) -> *mut FsNetworkHttpRequestParam {
        &mut self.ffi as *mut _
    }
}

fn keep_params_alive(id: FsNetworkRequestId, params: OwnedFfiParams) {
    PARAMS.with(|m| m.borrow_mut().insert(id, params));
}

fn drop_params(id: FsNetworkRequestId) {
    PARAMS.with(|m| m.borrow_mut().remove(&id));
}

#[derive(Default)]
pub struct HttpParams {
    pub headers: Vec<String>,
    pub post_field: Option<String>,
    pub body: Vec<u8>,
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
    on_done: impl FnOnce(HttpResponse) + 'static,
) -> NetResult<FsNetworkRequestId> {
    let mut owned = OwnedFfiParams::new(url, params)?;

    let id = unsafe {
        match method {
            Method::Get => fsNetworkHttpRequestGet(
                owned.url_ptr(),
                owned.ffi_ptr(),
                Some(http_trampoline),
                std::ptr::null_mut(),
            ),
            Method::Post => fsNetworkHttpRequestPost(
                owned.url_ptr(),
                owned.ffi_ptr(),
                Some(http_trampoline),
                std::ptr::null_mut(),
            ),
            Method::Put => fsNetworkHttpRequestPut(
                owned.url_ptr(),
                owned.ffi_ptr(),
                Some(http_trampoline),
                std::ptr::null_mut(),
            ),
        }
    };

    keep_params_alive(id, owned);
    HANDLERS.with(|m| m.borrow_mut().insert(id, Box::new(on_done)));

    Ok(id)
}

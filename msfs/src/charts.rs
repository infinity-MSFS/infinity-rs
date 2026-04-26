//! Safe wrapper around the MSFS `fsCharts*` API.
//!
//! The host Charts API is asynchronous: each request takes a C callback +
//! `void* userData`. We keep that shape but lift it into Rust by boxing a
//! `FnOnce` closure as the user data and trampolining through it.
//!
//! Owned wrappers ([`ChartIndex`], [`ChartPages`]) drop the host allocations
//! via `fsChartsFreeChartIndex` / `fsChartsFreeChartPages`. Borrowed accessor
//! methods (e.g. [`ChartIndex::categories`]) reinterpret the underlying C
//! arrays without copying — strings come back as `&str` borrowed from the
//! owning wrapper.
//!
//! Page images come back as a host image id (a `c_int`) suitable for use
//! with NanoVG via [`ChartImage::nvg_pattern`].

use crate::context::Context;
use crate::nvg::NvgContext;
use crate::sys;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartError {
    /// Requested chart GUID does not exist.
    NotFound,
    /// Provider name was not recognized.
    UnknownProvider,
    /// Transient network failure — safe to retry.
    NetworkError,
    /// Internal host error — do not retry.
    InternalError,
    /// An unrecognized error code was returned by the host.
    Unknown(u8),
}

impl ChartError {
    fn from_raw(raw: sys::FsChartError) -> Option<Self> {
        // `FsChartError_None` (0) is mapped to `Ok(_)`, not an error.
        match raw as u32 {
            0 => None,                              // None
            1 => Some(Self::NotFound),
            2 => Some(Self::UnknownProvider),
            3 => Some(Self::NetworkError),
            4 => Some(Self::InternalError),
            other => Some(Self::Unknown(other as u8)),
        }
    }
}

impl std::fmt::Display for ChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("chart not found"),
            Self::UnknownProvider => f.write_str("unknown chart provider"),
            Self::NetworkError => f.write_str("network error (retryable)"),
            Self::InternalError => f.write_str("internal chart error"),
            Self::Unknown(c) => write!(f, "unknown chart error ({c})"),
        }
    }
}

impl std::error::Error for ChartError {}

pub type ChartResult<T> = Result<T, ChartError>;

/// Build an [`sys::FsIcao`] from string fields. Truncated to the C buffer
/// sizes (`type`: 1 char, `region`: 2 + NUL, `airport`/`ident`: 8 + NUL).
pub fn make_icao(kind: char, region: &str, airport: &str, ident: &str) -> sys::FsIcao {
    let mut icao: sys::FsIcao = unsafe { std::mem::zeroed() };
    icao.type_ = kind as c_char;
    copy_into_buf(&mut icao.region, region);
    copy_into_buf(&mut icao.airport, airport);
    copy_into_buf(&mut icao.ident, ident);
    icao
}

fn copy_into_buf(dst: &mut [c_char], src: &str) {
    let bytes = src.as_bytes();
    let max = dst.len().saturating_sub(1); // leave NUL
    let n = bytes.len().min(max);
    for (i, b) in bytes.iter().take(n).enumerate() {
        dst[i] = *b as c_char;
    }
    if n < dst.len() {
        dst[n] = 0;
    }
}

// -- Owned host allocations ----------------------------------------------------

/// Owned `FsChartIndex` returned by [`get_index`].
///
/// Dropped via `fsChartsFreeChartIndex`.
pub struct ChartIndex {
    raw: *mut sys::FsChartIndex,
}

unsafe impl Send for ChartIndex {}

impl ChartIndex {
    /// ICAO of the airport this index belongs to.
    pub fn airport_icao(&self) -> &sys::FsIcao {
        unsafe { &(*self.raw).airportIcao }
    }

    /// Category slice borrowed from the underlying allocation.
    pub fn categories(&self) -> &[sys::FsChartIndexCategory] {
        unsafe {
            let r = &*self.raw;
            if r.chartCategories.is_null() || r.numChartCategories <= 0 {
                &[]
            } else {
                std::slice::from_raw_parts(r.chartCategories, r.numChartCategories as usize)
            }
        }
    }

    /// Iterate the categories with safe Rust accessors.
    pub fn iter_categories(&self) -> impl Iterator<Item = ChartCategoryRef<'_>> {
        self.categories().iter().map(|c| ChartCategoryRef { raw: c })
    }
}

impl Drop for ChartIndex {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sys::fsChartsFreeChartIndex(self.raw) };
            self.raw = std::ptr::null_mut();
        }
    }
}

#[derive(Clone, Copy)]
pub struct ChartCategoryRef<'a> {
    raw: &'a sys::FsChartIndexCategory,
}

impl<'a> ChartCategoryRef<'a> {
    pub fn name(self) -> &'a str {
        unsafe { c_str_to_borrowed(self.raw.name) }
    }

    pub fn charts(self) -> &'a [sys::FsChartMetadata] {
        unsafe {
            if self.raw.charts.is_null() || self.raw.numCharts <= 0 {
                &[]
            } else {
                std::slice::from_raw_parts(self.raw.charts, self.raw.numCharts as usize)
            }
        }
    }

    pub fn iter_charts(self) -> impl Iterator<Item = ChartMetadataRef<'a>> {
        self.charts().iter().map(|c| ChartMetadataRef { raw: c })
    }
}

#[derive(Clone, Copy)]
pub struct ChartMetadataRef<'a> {
    raw: &'a sys::FsChartMetadata,
}

impl<'a> ChartMetadataRef<'a> {
    pub fn guid(&self) -> &'a str {
        unsafe { c_str_to_borrowed(self.raw.guid) }
    }

    pub fn name(&self) -> &'a str {
        unsafe { c_str_to_borrowed(self.raw.name) }
    }

    pub fn provider(&self) -> &'a str {
        unsafe { c_str_to_borrowed(self.raw.provider) }
    }

    pub fn chart_type(&self) -> &'a str {
        unsafe { c_str_to_borrowed(self.raw.type_) }
    }

    pub fn airport_icao(&self) -> &'a sys::FsIcao {
        &self.raw.airportIcao
    }

    pub fn valid_from(&self) -> u64 {
        self.raw.validFrom
    }

    pub fn valid_until(&self) -> u64 {
        self.raw.validUntil
    }

    pub fn geo_referenced(&self) -> bool {
        self.raw.geoReferenced
    }
}

/// Owned `FsChartPages` returned by [`get_pages`].
pub struct ChartPages {
    raw: *mut sys::FsChartPages,
}

unsafe impl Send for ChartPages {}

impl ChartPages {
    pub fn pages(&self) -> &[sys::FsChartPage] {
        unsafe {
            let r = &*self.raw;
            if r.pages.is_null() || r.numPages <= 0 {
                &[]
            } else {
                std::slice::from_raw_parts(r.pages, r.numPages as usize)
            }
        }
    }

    pub fn iter_pages(&self) -> impl Iterator<Item = ChartPageRef<'_>> {
        self.pages().iter().map(|p| ChartPageRef { raw: p })
    }
}

impl Drop for ChartPages {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sys::fsChartsFreeChartPages(self.raw) };
            self.raw = std::ptr::null_mut();
        }
    }
}

#[derive(Clone, Copy)]
pub struct ChartPageRef<'a> {
    raw: &'a sys::FsChartPage,
}

impl<'a> ChartPageRef<'a> {
    pub fn width(self) -> u32 {
        self.raw.width
    }

    pub fn height(self) -> u32 {
        self.raw.height
    }

    pub fn geo_referenced(self) -> bool {
        self.raw.geoReferenced
    }

    pub fn urls(self) -> &'a [sys::FsChartPageUrl] {
        unsafe {
            if self.raw.urls.is_null() || self.raw.numUrls <= 0 {
                &[]
            } else {
                std::slice::from_raw_parts(self.raw.urls, self.raw.numUrls as usize)
            }
        }
    }

    /// Borrow the `(name, url)` pairs as Rust strings.
    pub fn iter_urls(self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.urls().iter().map(|u| unsafe {
            (c_str_to_borrowed(u.name), c_str_to_borrowed(u.url))
        })
    }

    pub fn areas(self) -> &'a [sys::FsChartArea] {
        unsafe {
            if self.raw.areas.is_null() || self.raw.numAreas <= 0 {
                &[]
            } else {
                std::slice::from_raw_parts(self.raw.areas, self.raw.numAreas as usize)
            }
        }
    }
}

/// Loaded chart page image. Owns a host image id usable with NanoVG.
///
/// Note: the SDK does not document an explicit free function for the image
/// id. Treat it as an NVG image and call [`ChartImage::delete_with_nvg`] when
/// you no longer need it (or just let the gauge tear-down release the
/// underlying NVG context).
pub struct ChartImage {
    id: c_int,
}

impl ChartImage {
    /// Raw page image id returned by `fsChartsGetPageImage`.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Build an `NVGpaint` sampling this image across `(x, y, w, h)`.
    pub fn nvg_pattern(
        &self,
        nvg: &NvgContext,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f32,
        alpha: f32,
    ) -> sys::NVGpaint {
        unsafe { sys::nvgImagePattern(nvg.raw(), x, y, w, h, angle, self.id, alpha) }
    }

    /// Query the image dimensions through NanoVG.
    pub fn size(&self, nvg: &NvgContext) -> (i32, i32) {
        nvg.image_size(self.id)
    }

    /// Release the underlying texture via `nvgDeleteImage`.
    pub fn delete_with_nvg(self, nvg: &NvgContext) {
        nvg.delete_image(self.id);
        // Prevent any future double-free if a Drop impl is added later.
        std::mem::forget(self);
    }
}

// -- Async request plumbing ----------------------------------------------------

unsafe fn c_str_to_borrowed<'a>(p: *const c_char) -> &'a str {
    if p.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("")
}

/// Reclaim the boxed callback that we stashed in `userData`, run it with the
/// host result, then drop the box.
unsafe fn invoke_callback<T, F>(user_data: *mut c_void, payload: ChartResult<T>)
where
    F: FnOnce(ChartResult<T>) + 'static,
{
    if user_data.is_null() {
        return;
    }
    let cb: Box<F> = unsafe { Box::from_raw(user_data as *mut F) };
    cb(payload);
}

extern "C" fn index_trampoline<F>(
    error: sys::FsChartError,
    index: *mut sys::FsChartIndex,
    user_data: *mut c_void,
) where
    F: FnOnce(ChartResult<ChartIndex>) + 'static,
{
    let result = match ChartError::from_raw(error) {
        Some(err) => Err(err),
        None if !index.is_null() => Ok(ChartIndex { raw: index }),
        None => Err(ChartError::InternalError),
    };
    unsafe { invoke_callback::<ChartIndex, F>(user_data, result) };
}

extern "C" fn pages_trampoline<F>(
    error: sys::FsChartError,
    pages: *mut sys::FsChartPages,
    user_data: *mut c_void,
) where
    F: FnOnce(ChartResult<ChartPages>) + 'static,
{
    let result = match ChartError::from_raw(error) {
        Some(err) => Err(err),
        None if !pages.is_null() => Ok(ChartPages { raw: pages }),
        None => Err(ChartError::InternalError),
    };
    unsafe { invoke_callback::<ChartPages, F>(user_data, result) };
}

extern "C" fn page_image_trampoline<F>(
    error: sys::FsChartError,
    image_id: c_int,
    user_data: *mut c_void,
) where
    F: FnOnce(ChartResult<ChartImage>) + 'static,
{
    let result = match ChartError::from_raw(error) {
        Some(err) => Err(err),
        None => Ok(ChartImage { id: image_id }),
    };
    unsafe { invoke_callback::<ChartImage, F>(user_data, result) };
}

/// Request the chart index for `airport` from `provider`.
///
/// `callback` runs on the host's reply thread (single-threaded inside the
/// WASM gauge runtime). Returns `Err` synchronously if the host refused to
/// queue the request — the callback will not be invoked in that case.
pub fn get_index<F>(airport: sys::FsIcao, provider: &str, callback: F) -> Result<(), ChartError>
where
    F: FnOnce(ChartResult<ChartIndex>) + 'static,
{
    let provider_c = CString::new(provider).map_err(|_| ChartError::InternalError)?;
    let user_data = Box::into_raw(Box::new(callback)) as *mut c_void;
    let ok = unsafe {
        sys::fsChartsGetIndex(
            airport,
            provider_c.as_ptr(),
            Some(index_trampoline::<F>),
            user_data,
        )
    };
    if ok {
        Ok(())
    } else {
        // Reclaim the box so we don't leak the closure.
        let _: Box<F> = unsafe { Box::from_raw(user_data as *mut F) };
        Err(ChartError::InternalError)
    }
}

/// Request the page list for a chart by its GUID.
pub fn get_pages<F>(chart_guid: &str, callback: F) -> Result<(), ChartError>
where
    F: FnOnce(ChartResult<ChartPages>) + 'static,
{
    let guid_c = CString::new(chart_guid).map_err(|_| ChartError::InternalError)?;
    let user_data = Box::into_raw(Box::new(callback)) as *mut c_void;
    let ok = unsafe {
        sys::fsChartsGetPages(guid_c.as_ptr(), Some(pages_trampoline::<F>), user_data)
    };
    if ok {
        Ok(())
    } else {
        let _: Box<F> = unsafe { Box::from_raw(user_data as *mut F) };
        Err(ChartError::InternalError)
    }
}

/// Request a chart page image. The `url` is the identifier returned by
/// [`ChartPageRef::iter_urls`].
pub fn get_page_image<F>(ctx: &Context, url: &str, callback: F) -> Result<(), ChartError>
where
    F: FnOnce(ChartResult<ChartImage>) + 'static,
{
    let url_c = CString::new(url).map_err(|_| ChartError::InternalError)?;
    let user_data = Box::into_raw(Box::new(callback)) as *mut c_void;
    let ok = unsafe {
        sys::fsChartsGetPageImage(
            ctx.fs_context(),
            url_c.as_ptr(),
            Some(page_image_trampoline::<F>),
            user_data,
        )
    };
    if ok {
        Ok(())
    } else {
        let _: Box<F> = unsafe { Box::from_raw(user_data as *mut F) };
        Err(ChartError::InternalError)
    }
}

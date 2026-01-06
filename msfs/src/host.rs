use core::ffi::c_char;

/// C ABI matches the C++ `GaugeHostApi` table.
///
/// Used for native testing (win32/linux64) where MSFS APIs aren't available.
/// A host harness calls `Gauge_SetHostApi` once to provide implementations.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GaugeHostApi {
    pub get_units_enum: Option<extern "C" fn(name: *const c_char) -> i32>,
    pub get_aircraft_var_enum: Option<extern "C" fn(name: *const c_char) -> i32>,
    pub aircraft_varget: Option<extern "C" fn(var: i32, unit: i32, index: i32) -> f64>,

    pub resolve_asset_path: Option<extern "C" fn(relative: *const c_char) -> *const c_char>,
}

static mut G_API: *const GaugeHostApi = core::ptr::null();

/// Exported entry point for host to inject the API table.
///
/// Mirrors your C++:
/// `MSFS_CALLBACK void Gauge_SetHostApi(const GaugeHostApi* api);`
#[unsafe(no_mangle)]
pub extern "C" fn Gauge_SetHostApi(api: *const GaugeHostApi) {
    unsafe {
        G_API = api;
    }
}

#[inline]
fn api() -> Option<&'static GaugeHostApi> {
    unsafe { G_API.as_ref() }
}

/// Thin wrappers that mirror the C++ `gauge_host_compat.h` helpers.

#[inline]
pub fn get_units_enum(name: *const c_char) -> i32 {
    api()
        .and_then(|a| a.get_units_enum)
        .map(|f| f(name))
        .unwrap_or(0)
}

#[inline]
pub fn get_aircraft_var_enum(name: *const c_char) -> i32 {
    api()
        .and_then(|a| a.get_aircraft_var_enum)
        .map(|f| f(name))
        .unwrap_or(0)
}

#[inline]
pub fn aircraft_varget(var: i32, unit: i32, index: i32) -> f64 {
    api()
        .and_then(|a| a.aircraft_varget)
        .map(|f| f(var, unit, index))
        .unwrap_or(0.0)
}

#[inline]
pub fn resolve_asset_path(relative: *const c_char) -> *const c_char {
    api()
        .and_then(|a| a.resolve_asset_path)
        .map(|f| f(relative))
        .unwrap_or(relative)
}

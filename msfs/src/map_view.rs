//! Safe wrapper around the MSFS `fsMapView*` API.
//!
//! A [`MapView`] owns a host-side map texture identified by an [`sys::FsTextureId`].
//! Configure it (mode, view, weather radar, etc.) and draw the resulting texture
//! through NanoVG using [`MapView::image_pattern`] together with
//! [`crate::nvg::NvgContext::fill_paint`]:
//!
//! ```rust
//! let map = MapView::new(ctx, 512, 512, RenderImageFlags::NONE)?;
//! map.set_view_mode(ViewMode::Aerial);
//! map.set_2d_view_lat_long(47.5, -122.3);
//! map.set_2d_view_radius_meters(5_000.0);
//! map.set_visibility(true);
//!
//! nvg.frame(w, h, dpr, |nvg| {
//!     nvg.begin_path();
//!     nvg.rect(0.0, 0.0, 512.0, 512.0);
//!     nvg.fill_paint(map.image_pattern(nvg, 0.0, 0.0, 512.0, 512.0, 0.0, 1.0));
//!     nvg.fill();
//! });
//! ```

use crate::context::Context;
use crate::nvg::{Color, NvgContext};
use crate::sys;

/// Bitfield wrapper for `FsRenderImageFlags`.
///
/// The MSFS SDK does not export named constants for these flags in the
/// public headers, so this is a thin newtype over the underlying `c_int`
/// for forward compatibility. Pass [`RenderImageFlags::NONE`] unless you
/// have a specific value in mind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderImageFlags(pub i32);

impl RenderImageFlags {
    pub const NONE: Self = Self(0);

    #[inline]
    pub fn raw(self) -> sys::FsRenderImageFlags {
        self.0 as sys::FsRenderImageFlags
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ViewMode {
    Aerial = 0,
    Altitude = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AltitudeReference {
    Geoid = 0,
    Plane = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum WeatherRadarMode {
    TopView = 0,
    Horizontal = 1,
    Vertical = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum View3DOrientation {
    FrontView = 0,
    TopView = 1,
    Custom = 2,
}

/// Layout-compatible with the C `FsRainRateColor` struct.
///
/// `Color` is `#[repr(C)]` with four `f32` fields matching `FsColor`'s
/// `{r,g,b,a}` member layout, so a `&[RainRateColor]` can be passed
/// directly to the C API as `*const FsRainRateColor`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct RainRateColor {
    pub color: Color,
    pub rain_rate: f32,
}

/// Owned handle to a map view texture created via `fsMapViewCreate`.
///
/// The underlying texture is released in [`Drop`] via `fsMapViewDelete`.
pub struct MapView {
    fs_ctx: sys::FsContext,
    id: sys::FsTextureId,
}

// MapView holds an opaque host handle (an integer id and an FsContext).
// Same single-threaded WASM rationale as `NvgContext`.
unsafe impl Send for MapView {}

// Lifecycle
impl MapView {
    /// Create a new map view backed by a texture of `width` x `height`.
    ///
    /// Returns `None` if the host failed to allocate a texture.
    pub fn new(ctx: &Context, width: u32, height: u32, flags: RenderImageFlags) -> Option<Self> {
        unsafe { Self::from_fs_context(ctx.fs_context(), width, height, flags) }
    }

    /// Create from a raw `FsContext`.
    ///
    /// # Safety
    /// `fs_ctx` must be a valid `FsContext` that outlives this `MapView`.
    pub unsafe fn from_fs_context(
        fs_ctx: sys::FsContext,
        width: u32,
        height: u32,
        flags: RenderImageFlags,
    ) -> Option<Self> {
        let id = unsafe { sys::fsMapViewCreate(fs_ctx, width, height, flags.raw()) };
        if id <= 0 {
            None
        } else {
            Some(Self { fs_ctx, id })
        }
    }

    /// Raw `FsTextureId` for this map view. Useful with NanoVG image APIs.
    #[inline]
    pub fn texture_id(&self) -> sys::FsTextureId {
        self.id
    }

    /// Build an `NVGpaint` that samples this map view as a textured pattern.
    ///
    /// Convenience over [`NvgContext`]'s raw FFI call to `nvgImagePattern`,
    /// passing this map view's texture id directly.
    pub fn image_pattern(
        &self,
        nvg: &NvgContext,
        ox: f32,
        oy: f32,
        ex: f32,
        ey: f32,
        angle: f32,
        alpha: f32,
    ) -> sys::NVGpaint {
        unsafe { sys::nvgImagePattern(nvg.raw(), ox, oy, ex, ey, angle, self.id, alpha) }
    }
}

impl Drop for MapView {
    fn drop(&mut self) {
        if self.id > 0 {
            unsafe { sys::fsMapViewDelete(self.fs_ctx, self.id) };
            self.id = 0;
        }
    }
}

// Common configuration. Each setter returns the underlying `bool` from the
// host so the caller can detect a rejected call (e.g. invalid id) without
// panicking. Most code can ignore the result.
impl MapView {
    pub fn set_visibility(&self, visible: bool) -> bool {
        unsafe { sys::fsMapViewSetVisibility(self.fs_ctx, self.id, visible) }
    }

    pub fn set_view_mode(&self, mode: ViewMode) -> bool {
        unsafe { sys::fsMapViewSetViewMode(self.fs_ctx, self.id, mode as _) }
    }

    pub fn set_size(&self, width: u32, height: u32) -> bool {
        unsafe { sys::fsMapViewSetSize(self.fs_ctx, self.id, width, height) }
    }

    pub fn set_background_color(&self, color: Color) -> bool {
        unsafe { sys::fsMapViewSetBackgroundColor(self.fs_ctx, self.id, color.into_raw()) }
    }

    /// Set the gradient ramp used in [`ViewMode::Altitude`].
    pub fn set_altitude_color_list(&self, colors: &[Color]) -> bool {
        // `Color` is `#[repr(C)]` with the same field layout as `FsColor`,
        // so the slice can be reinterpreted in-place.
        let ptr = colors.as_ptr() as *mut sys::FsColor;
        unsafe {
            sys::fsMapViewSetAltitudeColorList(self.fs_ctx, self.id, ptr, colors.len() as u32)
        }
    }

    pub fn set_altitude_reference(&self, reference: AltitudeReference) -> bool {
        unsafe { sys::fsMapViewSetAltitudeReference(self.fs_ctx, self.id, reference as _) }
    }

    pub fn set_altitude_range_feet(&self, min: f64, max: f64) -> bool {
        unsafe { sys::fsMapViewSetAltitudeRangeInFeet(self.fs_ctx, self.id, min, max) }
    }

    // -- Weather Radar Settings --
    pub fn set_weather_radar_bank_limits_radians(&self, min: f32, max: f32) -> bool {
        unsafe { sys::fsMapViewSetWeatherRadarBankLimitsInRadians(self.fs_ctx, self.id, min, max) }
    }

    pub fn set_weather_radar_cone_angle_radians(&self, angle: f32) -> bool {
        unsafe { sys::fsMapViewSetWeatherRadarConeAngleInRadians(self.fs_ctx, self.id, angle) }
    }

    pub fn set_weather_radar_mode(&self, mode: WeatherRadarMode) -> bool {
        unsafe { sys::fsMapViewSetWeatherRadarMode(self.fs_ctx, self.id, mode as _) }
    }

    pub fn set_weather_radar_pitch_limits_radians(&self, min: f32, max: f32) -> bool {
        unsafe { sys::fsMapViewSetWeatherRadarPitchLimitsInRadians(self.fs_ctx, self.id, min, max) }
    }

    pub fn set_weather_radar_rain_colors(&self, colors: &[RainRateColor]) -> bool {
        let ptr = colors.as_ptr() as *mut sys::FsRainRateColor;
        unsafe {
            sys::fsMapViewSetWeatherRadarRainColors(self.fs_ctx, self.id, ptr, colors.len() as u32)
        }
    }

    pub fn set_weather_radar_scan_rate(&self, rate: f32) -> bool {
        unsafe { sys::fsMapViewSetWeatherRadarScanRate(self.fs_ctx, self.id, rate) }
    }

    pub fn set_weather_radar_stabilization(&self, pitch: bool, bank: bool) -> bool {
        unsafe { sys::fsMapViewSetWeatherRadarStabilization(self.fs_ctx, self.id, pitch, bank) }
    }

    pub fn set_weather_radar_tilt_radians(&self, pitch: f64, bank: f64, heading: f64) -> bool {
        unsafe {
            sys::fsMapViewSetWeatherRadarTiltInRadians(self.fs_ctx, self.id, pitch, bank, heading)
        }
    }

    pub fn set_weather_radar_visibility(&self, visible: bool) -> bool {
        unsafe { sys::fsMapViewSetWeatherRadarVisibility(self.fs_ctx, self.id, visible) }
    }

    // -- Map Settings --

    pub fn set_map_isolines_visibility(&self, visible: bool) -> bool {
        unsafe { sys::fsMapViewSetMapIsolinesVisibility(self.fs_ctx, self.id, visible) }
    }

    pub fn set_3d(&self, enabled: bool) -> bool {
        unsafe { sys::fsMapViewSet3D(self.fs_ctx, self.id, enabled) }
    }

    pub fn set_2d_view_lat_long(&self, latitude: f64, longitude: f64) -> bool {
        unsafe { sys::fsMapViewSet2DViewLatLong(self.fs_ctx, self.id, latitude, longitude) }
    }

    pub fn set_2d_view_radius_meters(&self, radius: f32) -> bool {
        unsafe { sys::fsMapViewSet2DViewRadiusInMeters(self.fs_ctx, self.id, radius) }
    }

    pub fn set_2d_view_follow_mode(&self, follow: bool) -> bool {
        unsafe { sys::fsMapViewSet2DViewFollowMode(self.fs_ctx, self.id, follow) }
    }

    pub fn set_3d_view_orientation(&self, orientation: View3DOrientation) -> bool {
        unsafe { sys::fsMapViewSet3DViewOrientation(self.fs_ctx, self.id, orientation as _) }
    }

    pub fn set_3d_custom_view_orientation_radians(
        &self,
        pitch: f64,
        bank: f64,
        heading: f64,
    ) -> bool {
        unsafe {
            sys::fsMapViewSet3DCustomViewOrientationInRadians(
                self.fs_ctx,
                self.id,
                pitch,
                bank,
                heading,
            )
        }
    }
}

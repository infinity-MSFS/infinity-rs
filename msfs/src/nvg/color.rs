use crate::sys;
use std::ops;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgbaf(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgbaf(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::rgbaf(1.0, 1.0, 1.0, 1.0);
    pub const RED: Self = Self::rgbaf(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::rgbaf(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::rgbaf(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::rgbaf(1.0, 1.0, 0.0, 1.0);
    pub const CYAN: Self = Self::rgbaf(0.0, 1.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::rgbaf(1.0, 0.0, 1.0, 1.0);
}

impl Color {
    /// Create from `u8` RGB values. Alpha defaults to 255 (opaque).
    #[inline]
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Create from `u8` RGBA values.
    #[inline]
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Create from `f32` RGB values in `[0.0, 1.0]`. Alpha defaults to 1.0.
    #[inline]
    pub fn rgbf(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create from `f32` RGBA values in `[0.0, 1.0]`.
    #[inline]
    pub const fn rgbaf(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from a packed `0xRRGGBBAA` hex value.
    ///
    /// ```rust
    /// let coral = Color::hex(0xFF7F50FF);
    /// let semi_white = Color::hex(0xFFFFFF80);
    /// ```
    #[inline]
    pub fn hex(rgba: u32) -> Self {
        Self::rgba(
            ((rgba >> 24) & 0xFF) as u8,
            ((rgba >> 16) & 0xFF) as u8,
            ((rgba >> 8) & 0xFF) as u8,
            (rgba & 0xFF) as u8,
        )
    }

    /// Create from a `#RRGGBB` or `#RRGGBBAA` CSS-style hex string.
    ///
    /// ```rust
    /// let c = Color::css("#FF7F50").unwrap();
    /// ```
    pub fn css(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        match s.len() {
            6 => {
                let v = u32::from_str_radix(s, 16).ok()?;
                Some(Self::hex((v << 8) | 0xFF))
            }
            8 => {
                let v = u32::from_str_radix(s, 16).ok()?;
                Some(Self::hex(v))
            }
            _ => None,
        }
    }

    /// Create from HSL. All values in `[0.0, 1.0]`. Alpha defaults to 1.0.
    #[inline]
    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        unsafe { std::mem::transmute(sys::nvgHSL(h, s, l)) }
    }

    /// Create from HSLA. HSL in `[0.0, 1.0]`, alpha in `[0, 255]`.
    #[inline]
    pub fn hsla(h: f32, s: f32, l: f32, a: u8) -> Self {
        unsafe { std::mem::transmute(sys::nvgHSLA(h, s, l, a)) }
    }
}

impl Color {
    /// Return a copy with a different alpha (`0.0`–`1.0`).
    #[inline]
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Linearly interpolate between `self` and `other` by factor `t` in `[0.0, 1.0]`.
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        unsafe { std::mem::transmute(sys::nvgLerpRGBA(self.into_raw(), other.into_raw(), t)) }
    }

    /// Darken by a factor (`0.0` = black, `1.0` = unchanged).
    #[inline]
    pub fn darken(self, factor: f32) -> Self {
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
            a: self.a,
        }
    }

    /// Lighten by mixing toward white by `factor` (`0.0` = unchanged, `1.0` = white).
    #[inline]
    pub fn lighten(self, factor: f32) -> Self {
        self.lerp(Color::WHITE.with_alpha(self.a), factor)
    }
}

impl Color {
    #[inline]
    pub(crate) fn into_raw(self) -> sys::NVGcolor {
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub(crate) fn from_raw(raw: sys::NVGcolor) -> Self {
        unsafe { std::mem::transmute(raw) }
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Self::rgba(r, g, b, a)
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        Self::hex(hex)
    }
}

pub mod nvg {
    use crate::sys;

    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct Color(sys::NVGcolor);

    impl Color {
        #[inline]
        pub fn rgb(r: u8, g: u8, b: u8) -> Self {
            unsafe { Self(sys::nvgRGB(r, g, b)) }
        }
        #[inline]
        pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
            unsafe { Self(sys::nvgRGBA(r, g, b, a)) }
        }
        #[inline]
        pub fn rgbf(r: f32, g: f32, b: f32) -> Self {
            unsafe { Self(sys::nvgRGBf(r, g, b)) }
        }
        #[inline]
        pub fn rgbaf(r: f32, g: f32, b: f32, a: f32) -> Self {
            unsafe { Self(sys::nvgRGBAf(r, g, b, a)) }
        }

        #[inline]
        pub fn hex(rgb: u32) -> Self {
            let r = ((rgb >> 16) & 0xff) as u8;
            let g = ((rgb >> 8) & 0xff) as u8;
            let b = (r & 0xff) as u8;
            Self::rgb(r, g, b)
        }

        #[inline]
        pub fn alpha(self, a: f32) -> Self {
            unsafe { Self(sys::nvgTransRGBAf(self.0, a.clamp(0.0, 1.0))) }
        }

        #[inline] pub fn white() -> Self { Self::rgb(255,255,255) }
        #[inline] pub fn black() -> Self { Self::rgb(0,0,0) }

        #[inline] pub(crate) fn raw(self) -> sys::NVGcolor { self.0 }
    }
}

use crate::sys;

/// A 2×3 affine transformation matrix stored as `[a, b, c, d, e, f]`.
///
/// ```text
/// [a  c  e]     [sx  kx  tx]
/// [b  d  f]  =  [ky  sy  ty]
/// [0  0  1]     [ 0   0   1]
/// ```
///
/// Use the builder-style methods to chain transforms:
/// ```rust
/// let xform = Transform::identity()
///     .translate(100.0, 50.0)
///     .rotate(std::f32::consts::FRAC_PI_4)
///     .scale(2.0, 2.0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub m: [f32; 6],
}

impl Transform {
    /// Identity matrix (no transformation).
    pub fn identity() -> Self {
        let mut m = [0.0f32; 6];
        unsafe { sys::nvgTransformIdentity(m.as_mut_ptr()) };
        Self { m }
    }

    /// Pure translation.
    pub fn from_translate(tx: f32, ty: f32) -> Self {
        let mut m = [0.0f32; 6];
        unsafe { sys::nvgTransformTranslate(m.as_mut_ptr(), tx, ty) };
        Self { m }
    }

    /// Pure scale.
    pub fn from_scale(sx: f32, sy: f32) -> Self {
        let mut m = [0.0f32; 6];
        unsafe { sys::nvgTransformScale(m.as_mut_ptr(), sx, sy) };
        Self { m }
    }

    /// Pure rotation (radians).
    pub fn from_rotate(angle: f32) -> Self {
        let mut m = [0.0f32; 6];
        unsafe { sys::nvgTransformRotate(m.as_mut_ptr(), angle) };
        Self { m }
    }

    // Builders

    /// Append a translation: `self = self * T(tx, ty)`.
    pub fn translate(mut self, tx: f32, ty: f32) -> Self {
        let t = Self::from_translate(tx, ty);
        unsafe { sys::nvgTransformMultiply(self.m.as_mut_ptr(), t.m.as_ptr()) };
        self
    }

    /// Append a rotation: `self = self * R(angle)`.
    pub fn rotate(mut self, angle: f32) -> Self {
        let t = Self::from_rotate(angle);
        unsafe { sys::nvgTransformMultiply(self.m.as_mut_ptr(), t.m.as_ptr()) };
        self
    }

    /// Append a scale: `self = self * S(sx, sy)`.
    pub fn scale(mut self, sx: f32, sy: f32) -> Self {
        let t = Self::from_scale(sx, sy);
        unsafe { sys::nvgTransformMultiply(self.m.as_mut_ptr(), t.m.as_ptr()) };
        self
    }

    /// Append a skew-X.
    pub fn skew_x(mut self, angle: f32) -> Self {
        let mut t = [0.0f32; 6];
        unsafe { sys::nvgTransformSkewX(t.as_mut_ptr(), angle) };
        unsafe { sys::nvgTransformMultiply(self.m.as_mut_ptr(), t.as_ptr()) };
        self
    }

    /// Append a skew-Y.
    pub fn skew_y(mut self, angle: f32) -> Self {
        let mut t = [0.0f32; 6];
        unsafe { sys::nvgTransformSkewY(t.as_mut_ptr(), angle) };
        unsafe { sys::nvgTransformMultiply(self.m.as_mut_ptr(), t.as_ptr()) };
        self
    }

    /// Multiply: `self = self * other`.
    pub fn then(mut self, other: &Transform) -> Self {
        unsafe { sys::nvgTransformMultiply(self.m.as_mut_ptr(), other.m.as_ptr()) };
        self
    }

    /// Compute the inverse. Returns `None` if the matrix is singular.
    pub fn inverse(&self) -> Option<Self> {
        let mut m = [0.0f32; 6];
        let ok = unsafe { sys::nvgTransformInverse(m.as_mut_ptr(), self.m.as_ptr()) };
        if ok != 0 { Some(Self { m }) } else { None }
    }

    /// Transform a point.
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        unsafe { sys::nvgTransformPoint(&mut dx, &mut dy, self.m.as_ptr(), x, y) };
        (dx, dy)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[inline]
pub fn deg_to_rad(deg: f32) -> f32 {
    unsafe { sys::nvgDegToRad(deg) }
}

#[inline]
pub fn rad_to_deg(rad: f32) -> f32 {
    unsafe { sys::nvgRadToDeg(rad) }
}

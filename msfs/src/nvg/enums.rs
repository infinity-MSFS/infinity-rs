use std::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Winding {
    /// Counter-clockwise: used for solid shapes.
    Ccw = 1,
    /// Clockwise: used for holes.
    Cw = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Solidity {
    Solid = 1, // CCW
    Hole = 2,  // CW
}

impl From<Solidity> for Winding {
    fn from(s: Solidity) -> Self {
        match s {
            Solidity::Solid => Winding::Ccw,
            Solidity::Hole => Winding::Cw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LineCap {
    Butt = 0,
    Round = 1,
    Square = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LineJoin {
    Miter = 4,
    Round = 1,
    Bevel = 3,
}

/// TODO: move these to bitflags
/// Text alignment flags. Combine horizontal and vertical with `|`.
///
/// ```rust
/// ctx.text_align(Align::CENTER | Align::MIDDLE);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Align(pub i32);

impl Align {
    pub const LEFT: Self = Self(1 << 0);
    pub const CENTER: Self = Self(1 << 1);
    pub const RIGHT: Self = Self(1 << 2);
    pub const TOP: Self = Self(1 << 3);
    pub const MIDDLE: Self = Self(1 << 4);
    pub const BOTTOM: Self = Self(1 << 5);
    pub const BASELINE: Self = Self(1 << 6);
}

impl ops::BitOr for Align {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for Align {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl From<Align> for i32 {
    fn from(a: Align) -> i32 {
        a.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum CompositeOp {
    SourceOver = 0,
    SourceIn = 1,
    SourceOut = 2,
    Atop = 3,
    DestinationOver = 4,
    DestinationIn = 5,
    DestinationOut = 6,
    DestinationAtop = 7,
    Lighter = 8,
    Copy = 9,
    Xor = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum BlendFactor {
    Zero = 1 << 0,
    One = 1 << 1,
    SrcColor = 1 << 2,
    OneMinusSrcColor = 1 << 3,
    DstColor = 1 << 4,
    OneMinusDstColor = 1 << 5,
    SrcAlpha = 1 << 6,
    OneMinusSrcAlpha = 1 << 7,
    DstAlpha = 1 << 8,
    OneMinusDstAlpha = 1 << 9,
    SrcAlphaSaturate = 1 << 10,
}

/// TODO: move these to bitflags
/// Flags for image creation. Combine with `|`.
///
/// ```rust
/// let flags = ImageFlags::REPEAT_X | ImageFlags::REPEAT_Y;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageFlags(pub i32);

impl ImageFlags {
    pub const NONE: Self = Self(0);
    pub const GENERATE_MIPMAPS: Self = Self(1 << 0);
    pub const REPEAT_X: Self = Self(1 << 1);
    pub const REPEAT_Y: Self = Self(1 << 2);
    pub const FLIP_Y: Self = Self(1 << 3);
    pub const PREMULTIPLIED: Self = Self(1 << 4);
    pub const NEAREST: Self = Self(1 << 5);
}

impl ops::BitOr for ImageFlags {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for ImageFlags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ClipMode {
    Replace = 0,
    Intersect = 1,
    Union = 2,
    Xor = 3,
    Exclude = 4,
    Complement = 5,
    Ignore = 8,
    Use = 16,
}

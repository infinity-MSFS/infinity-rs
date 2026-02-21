use crate::nvg::context::NvgContext;
use crate::nvg::enums::{Solidity, Winding};

/// Chainable path construction.
///
/// You normally won't construct this directly — use `ctx.path()` instead:
///
/// ```rust
/// ctx.path()
///     .move_to(10.0, 10.0)
///     .line_to(200.0, 10.0)
///     .line_to(200.0, 100.0)
///     .close()
///     .winding(Winding::Ccw);
///
/// ctx.fill_color(Color::RED);
/// ctx.fill();
/// ```
///
/// Or combine with a [`Shape`](super::Shape) for a fully self-contained draw call.
pub struct PathBuilder<'a> {
    ctx: &'a NvgContext,
}

impl<'a> PathBuilder<'a> {
    pub(crate) fn new(ctx: &'a NvgContext) -> Self {
        Self { ctx }
    }

    /// Start a new sub-path at `(x, y)`.
    pub fn move_to(self, x: f32, y: f32) -> Self {
        self.ctx.move_to(x, y);
        self
    }

    /// Line from current point to `(x, y)`.
    pub fn line_to(self, x: f32, y: f32) -> Self {
        self.ctx.line_to(x, y);
        self
    }

    /// Cubic bezier from current point through `(c1x,c1y)`, `(c2x,c2y)` to `(x,y)`.
    pub fn bezier_to(self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self {
        self.ctx.bezier_to(c1x, c1y, c2x, c2y, x, y);
        self
    }

    /// Quadratic bezier through `(cx, cy)` to `(x, y)`.
    pub fn quad_to(self, cx: f32, cy: f32, x: f32, y: f32) -> Self {
        self.ctx.quad_to(cx, cy, x, y);
        self
    }

    /// Arc from current point toward `(x1,y1)` and `(x2,y2)` with given `radius`.
    pub fn arc_to(self, x1: f32, y1: f32, x2: f32, y2: f32, radius: f32) -> Self {
        self.ctx.arc_to(x1, y1, x2, y2, radius);
        self
    }

    /// Close the current sub-path with a line segment.
    pub fn close(self) -> Self {
        self.ctx.close_path();
        self
    }

    /// Set path winding direction.
    pub fn winding(self, dir: Winding) -> Self {
        self.ctx.path_winding(dir);
        self
    }

    /// Set path solidity (reads nicer than winding in some contexts).
    pub fn solidity(self, sol: Solidity) -> Self {
        self.ctx.path_winding(sol.into());
        self
    }

    // shapes within a path

    /// Add a rectangle sub-path.
    pub fn rect(self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.ctx.rect(x, y, w, h);
        self
    }

    /// Add a rounded rectangle sub-path.
    pub fn rounded_rect(self, x: f32, y: f32, w: f32, h: f32, r: f32) -> Self {
        self.ctx.rounded_rect(x, y, w, h, r);
        self
    }

    /// Add a circle sub-path.
    pub fn circle(self, cx: f32, cy: f32, r: f32) -> Self {
        self.ctx.circle(cx, cy, r);
        self
    }

    /// Add an ellipse sub-path.
    pub fn ellipse(self, cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        self.ctx.ellipse(cx, cy, rx, ry);
        self
    }

    /// Add a circular arc sub-path.
    pub fn arc(self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, dir: Winding) -> Self {
        self.ctx.arc(cx, cy, r, a0, a1, dir);
        self
    }
}

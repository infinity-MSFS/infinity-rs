use crate::nvg::color::Color;
use crate::nvg::context::NvgContext;
use crate::nvg::enums::Winding;
use crate::nvg::paint::FillStyle;

#[derive(Debug, Clone)]
enum Geometry {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    RoundedRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
    },
    RoundedRectVarying {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tl: f32,
        tr: f32,
        br: f32,
        bl: f32,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
    },
    Arc {
        cx: f32,
        cy: f32,
        r: f32,
        a0: f32,
        a1: f32,
        dir: Winding,
    },
    /// Arbitrary path defined by a closure.
    Custom(CustomPath),
}

#[derive(Clone)]
struct CustomPath(std::sync::Arc<dyn Fn(&NvgContext) + Send + Sync>);

impl std::fmt::Debug for CustomPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<custom path>")
    }
}

// Fill

#[derive(Clone)]
enum StylePaint {
    Solid(Color),
    Dynamic(std::sync::Arc<dyn FillStyle + Send + Sync>),
}

impl StylePaint {
    fn apply_fill(&self, ctx: &NvgContext) {
        match self {
            Self::Solid(c) => ctx.fill_color(*c),
            Self::Dynamic(p) => p.apply_fill(ctx),
        }
    }
    fn apply_stroke(&self, ctx: &NvgContext) {
        match self {
            Self::Solid(c) => ctx.stroke_color(*c),
            Self::Dynamic(p) => p.apply_stroke(ctx),
        }
    }
}

#[derive(Clone)]
struct StrokeStyle {
    paint: StylePaint,
    width: f32,
}

// Shape builder

/// A reusable, declarative shape definition.
///
/// Build a shape, configure its fill/stroke, then `.draw(ctx)`.
/// Shapes are `Clone + Send + Sync` so you can store them as constants
/// or share them across threads.
///
/// # Examples
///
/// ```rust
/// // Simple colored rectangle
/// Shape::rect(10.0, 10.0, 200.0, 60.0)
///     .fill(Color::hex(0x2196F3FF))
///     .draw(ctx);
///
/// // Rounded button with gradient fill and border
/// let btn = Shape::rounded_rect(0.0, 0.0, 180.0, 44.0, 6.0)
///     .fill(Gradient::linear(ctx, 0.0, 0.0, 0.0, 44.0,
///         Color::hex(0x42A5F5FF), Color::hex(0x1565C0FF)))
///     .stroke(Color::hex(0x0D47A1FF), 1.0);
/// btn.draw(ctx);
///
/// // Ring / donut using a custom path
/// Shape::custom(|ctx| {
///     ctx.circle(100.0, 100.0, 50.0);
///     ctx.path_winding(Winding::Ccw);
///     ctx.circle(100.0, 100.0, 30.0);
///     ctx.path_winding(Winding::Cw);
/// })
/// .fill(Color::WHITE)
/// .draw(ctx);
///
/// // Drop shadow behind a panel
/// let shadow = Shape::rect(12.0, 12.0, 200.0, 120.0)
///     .fill(Gradient::box_(ctx, 10.0, 10.0, 200.0, 120.0, 8.0, 16.0,
///         Color::BLACK.with_alpha(0.4), Color::TRANSPARENT));
/// shadow.draw(ctx);
/// ```
#[derive(Clone)]
pub struct Shape {
    geom: Geometry,
    fill: Option<StylePaint>,
    strokes: Vec<StrokeStyle>,
}

impl Shape {
    pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::with_geom(Geometry::Rect { x, y, w, h })
    }

    pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Self {
        Self::with_geom(Geometry::RoundedRect { x, y, w, h, r })
    }

    pub fn rounded_rect_varying(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tl: f32,
        tr: f32,
        br: f32,
        bl: f32,
    ) -> Self {
        Self::with_geom(Geometry::RoundedRectVarying {
            x,
            y,
            w,
            h,
            tl,
            tr,
            br,
            bl,
        })
    }

    pub fn circle(cx: f32, cy: f32, r: f32) -> Self {
        Self::with_geom(Geometry::Circle { cx, cy, r })
    }

    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        Self::with_geom(Geometry::Ellipse { cx, cy, rx, ry })
    }

    pub fn arc(cx: f32, cy: f32, r: f32, a0: f32, a1: f32, dir: Winding) -> Self {
        Self::with_geom(Geometry::Arc {
            cx,
            cy,
            r,
            a0,
            a1,
            dir,
        })
    }

    /// Arbitrary path. The closure receives the `NvgContext` *after* `begin_path`
    /// has been called — just add your sub-paths.
    pub fn custom<F>(f: F) -> Self
    where
        F: Fn(&NvgContext) + Send + Sync + 'static,
    {
        Self::with_geom(Geometry::Custom(CustomPath(std::sync::Arc::new(f))))
    }

    fn with_geom(geom: Geometry) -> Self {
        Self {
            geom,
            fill: None,
            strokes: Vec::new(),
        }
    }

    /// Set a solid color fill.
    pub fn fill(mut self, style: impl Into<ShapeFill>) -> Self {
        self.fill = Some(style.into().0);
        self
    }

    /// Add a stroke. Can be called multiple times for layered strokes.
    pub fn stroke(mut self, style: impl Into<ShapeFill>, width: f32) -> Self {
        self.strokes.push(StrokeStyle {
            paint: style.into().0,
            width,
        });
        self
    }

    /// Emit the shape to the NVG context.
    pub fn draw(&self, ctx: &NvgContext) {
        ctx.begin_path();
        self.emit_geometry(ctx);

        if let Some(ref fill) = self.fill {
            fill.apply_fill(ctx);
            ctx.fill();
        }

        for s in &self.strokes {
            ctx.stroke_width(s.width);
            s.paint.apply_stroke(ctx);
            ctx.stroke();
        }
    }

    fn emit_geometry(&self, ctx: &NvgContext) {
        match &self.geom {
            Geometry::Rect { x, y, w, h } => ctx.rect(*x, *y, *w, *h),
            Geometry::RoundedRect { x, y, w, h, r } => ctx.rounded_rect(*x, *y, *w, *h, *r),
            Geometry::RoundedRectVarying {
                x,
                y,
                w,
                h,
                tl,
                tr,
                br,
                bl,
            } => ctx.rounded_rect_varying(*x, *y, *w, *h, *tl, *tr, *br, *bl),
            Geometry::Circle { cx, cy, r } => ctx.circle(*cx, *cy, *r),
            Geometry::Ellipse { cx, cy, rx, ry } => ctx.ellipse(*cx, *cy, *rx, *ry),
            Geometry::Arc {
                cx,
                cy,
                r,
                a0,
                a1,
                dir,
            } => ctx.arc(*cx, *cy, *r, *a0, *a1, *dir),
            Geometry::Custom(CustomPath(f)) => f(ctx),
        }
    }
}

pub struct ShapeFill(StylePaint);

impl From<Color> for ShapeFill {
    fn from(c: Color) -> Self {
        ShapeFill(StylePaint::Solid(c))
    }
}

// Unstable on rustc 1.90.0, maybe revisit when specialization is stabilized because I would prefer this to be the way its written.
// impl<T: FillStyle + Send + Sync + 'static> From<T> for ShapeFill
// where
//     T: Clone,
// {
//     default fn from(t: T) -> Self {
//         ShapeFill(StylePaint::Dynamic(std::sync::Arc::new(t)))
//     }
// }

impl From<super::Gradient> for ShapeFill {
    fn from(g: super::Gradient) -> Self {
        ShapeFill(StylePaint::Dynamic(std::sync::Arc::new(g)))
    }
}

impl From<super::ImagePattern> for ShapeFill {
    fn from(p: super::ImagePattern) -> Self {
        ShapeFill(StylePaint::Dynamic(std::sync::Arc::new(p)))
    }
}

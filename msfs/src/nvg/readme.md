# NVG binding
These got a little complicated, I ended up binding literally everything so I feel like this needs its own readme, I'll do something similar to all the other modules in the future.

## Context Lifecycle

```rust
pub struct MyGauge {
    nvg: Option<NvgContext>,  // owns the NVG context
    font: Option<i32>,
}

impl Gauge for MyGauge {
    fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool {
        // Create from Context — wires up MSFS render backend automatically
        let nvg = NvgContext::new(ctx).expect("NVG init failed");
        self.font = nvg.create_font("sans", "./data/Roboto-Regular.ttf");
        self.nvg = Some(nvg);
        true
    }

    fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool {
        let nvg = self.nvg.as_ref().unwrap();
        let px_ratio = draw.fb_width() as f32 / draw.win_width();
        nvg.frame(draw.win_width(), draw.win_height(), px_ratio, |nvg| {
            // All drawing here
        });
        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        self.nvg = None;  // calls nvgDeleteInternal via Drop
        true
    }
}
```

---

## Three Ways to Draw

### 1. Direct (C-style, full control)
```rust
ctx.begin_path();
ctx.rounded_rect(10.0, 10.0, 200.0, 50.0, 8.0);
ctx.fill_color(Color::hex(0x2196F3FF));
ctx.fill();
ctx.stroke_color(Color::WHITE);
ctx.stroke_width(1.0);
ctx.stroke();
```

### 2. Path Builder (chainable, still manual fill/stroke)
```rust
ctx.path()
    .move_to(0.0, 0.0)
    .line_to(100.0, 0.0)
    .bezier_to(100.0, 50.0, 50.0, 80.0, 0.0, 50.0)
    .close();

ctx.fill_color(Color::YELLOW);
ctx.fill();
```

### 3. Shape Builder (declarative, self-contained, reusable)
```rust
Shape::rounded_rect(10.0, 10.0, 200.0, 50.0, 8.0)
    .fill(Color::hex(0x2196F3FF))
    .stroke(Color::WHITE, 1.0)
    .draw(ctx);
```

---

## Colors

```rust
Color::rgb(255, 128, 0)           // u8 RGB, alpha = 255
Color::rgba(255, 128, 0, 200)     // u8 RGBA
Color::rgbf(1.0, 0.5, 0.0)       // f32 RGB
Color::hex(0xFF8000FF)            // 0xRRGGBBAA
Color::css("#FF8000")             // CSS hex string
Color::hsl(0.08, 1.0, 0.5)       // HSL

Color::WHITE.with_alpha(0.5)      // modify alpha
Color::RED.darken(0.7)            // darken
Color::RED.lighten(0.3)           // lighten
Color::RED.lerp(Color::BLUE, 0.5) // interpolate
```

### Named constants
`BLACK`, `WHITE`, `RED`, `GREEN`, `BLUE`, `YELLOW`, `CYAN`, `MAGENTA`, `TRANSPARENT`

---

## Gradients & Patterns

```rust
// Linear gradient
let g = Gradient::linear(ctx, x0, y0, x1, y1, color_start, color_end);

// Radial gradient
let g = Gradient::radial(ctx, cx, cy, inner_r, outer_r, inner_col, outer_col);

// Box gradient (drop shadows, glows)
let g = Gradient::box_(ctx, x, y, w, h, corner_r, feather, inner_col, outer_col);

// Image pattern
let p = ImagePattern::new(ctx, ox, oy, tile_w, tile_h, angle, image_handle, alpha);

// Use as fill:
Shape::rect(0.0, 0.0, 200.0, 100.0).fill(g).draw(ctx);
```

---

## Transforms

```rust
// On context (imperative)
ctx.translate(100.0, 50.0);
ctx.rotate(deg_to_rad(45.0));
ctx.scale(2.0, 2.0);

// Standalone (composable)
let xf = Transform::identity()
    .translate(100.0, 50.0)
    .rotate(deg_to_rad(45.0))
    .scale(2.0, 2.0);
ctx.apply_transform(&xf);
let (px, py) = xf.apply(10.0, 20.0); // transform a point
```

---

## State Scoping

```rust
// Manual
ctx.save();
ctx.translate(50.0, 50.0);
// ...draw...
ctx.restore();

// Closure-based (preferred)
ctx.scoped(|ctx| {
    ctx.translate(50.0, 50.0);
    // ...draw...
}); // auto-restored
```

---

## Text

```rust
ctx.font_face("Roboto");
ctx.font_size(24.0);
ctx.fill_color(Color::WHITE);
ctx.text_align(Align::CENTER | Align::MIDDLE);
ctx.text(100.0, 50.0, "Hello");

// Wrapped text
ctx.text_box(10.0, 10.0, 200.0, "Long text that wraps...");

// Measure before drawing
let bounds = ctx.text_bounds(0.0, 0.0, "Hello");
println!("width={} height={}", bounds.width(), bounds.height());
```

---

## Shape Quick Reference

| Shape | Constructor |
|-------|-----------|
| Rectangle | `Shape::rect(x, y, w, h)` |
| Rounded rect | `Shape::rounded_rect(x, y, w, h, r)` |
| Varying corners | `Shape::rounded_rect_varying(x, y, w, h, tl, tr, br, bl)` |
| Circle | `Shape::circle(cx, cy, r)` |
| Ellipse | `Shape::ellipse(cx, cy, rx, ry)` |
| Arc | `Shape::arc(cx, cy, r, a0, a1, dir)` |
| Custom path | `Shape::custom(\|ctx\| { ... })` |

All shapes support `.fill(style)` and `.stroke(style, width)` before `.draw(ctx)`.

---

## Common Patterns

### Drop shadow
```rust
let shadow = Gradient::box_(ctx, x+2.0, y+2.0, w, h, r, 10.0,
    Color::BLACK.with_alpha(0.5), Color::TRANSPARENT);
Shape::rect(x-5.0, y-5.0, w+10.0, h+10.0).fill(shadow).draw(ctx);
```

### Donut / ring
```rust
Shape::custom(|ctx| {
    ctx.circle(cx, cy, outer_r);
    ctx.path_winding(Winding::Ccw);
    ctx.circle(cx, cy, inner_r);
    ctx.path_winding(Winding::Cw);
}).fill(Color::WHITE).draw(ctx);
```

### Clipped drawing
```rust
ctx.scoped(|ctx| {
    ctx.scissor(x, y, w, h);
    // everything here is clipped to the rect
});
```
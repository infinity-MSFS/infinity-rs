//! # Example: Attitude Indicator (Rust port of the C++ SDK sample)
//!
//! Shows the full NVG lifecycle with `NvgContext::new(&ctx)`.
//!
//! Compare with the Asobo C++ sample — the flow is identical:
//!   init  → nvgCreateInternal (via NvgContext::new)
//!   draw  → nvgBeginFrame / draw / nvgEndFrame
//!   kill  → nvgDeleteInternal (via Drop)
//!
//! For more details on the NVG API bindings, see the readme in the nvg module.

use msfs::nvg::*;
use msfs::prelude::*;
use std::f32::consts::PI;

pub struct AttitudeGauge {
    nvg: Option<NvgContext>,
    font: Option<i32>,

    pitch_var: AVar,
    bank_var: AVar,
}

impl AttitudeGauge {
    pub fn new() -> Self {
        Self {
            nvg: None,
            font: None,
            pitch_var: AVar::new("ATTITUDE INDICATOR PITCH DEGREES", "DEGREES")
                .expect("Failed to create pitch AVar"),
            bank_var: AVar::new("ATTITUDE INDICATOR BANK DEGREES", "DEGREES")
                .expect("Failed to create bank AVar"),
        }
    }
}

impl Gauge for AttitudeGauge {
    fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool {
        let nvg = match NvgContext::new(ctx) {
            Some(n) => n,
            None => return false,
        };

        self.font = nvg.create_font("sans", "./data/Roboto-Regular.ttf");
        self.nvg = Some(nvg);
        true
    }

    fn update(&mut self, ctx: &Context, dt: f32) -> bool {
        true
    }

    fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool {
        let nvg = match &self.nvg {
            Some(n) => n,
            None => return false,
        };

        let pitch = self.pitch_var.get().unwrap_or(0.0) as f32;
        let bank = self.bank_var.get().unwrap_or(0.0) as f32;

        let win_w = draw.winWidth as f32;
        let win_h = draw.winHeight as f32;

        let px_ratio = draw.fbWidth as f32 / win_w;
        let size = (win_w * win_w + win_h * win_h).sqrt() * 1.1;

        nvg.frame(win_w, win_h, px_ratio, |nvg| {
            let half = size * 0.5;
            let fh = half * (1.0 - (pitch * PI / 180.0).sin());

            // sky and ground
            nvg.scoped(|nvg| {
                nvg.translate(win_w * 0.5, win_h * 0.5);
                nvg.rotate(bank * PI / 180.0);

                Shape::rect(-half, -half, size, fh)
                    .fill(Color::rgb(0, 191, 255))
                    .draw(nvg);
                Shape::rect(-half, -half + fh, size, size - fh)
                    .fill(Color::rgb(210, 103, 30))
                    .draw(nvg);
            });

            // Attitude fixed airplane symbol
            nvg.scoped(|nvg| {
                nvg.translate(win_w * 0.5, win_h * 0.5);

                nvg.stroke_color(Color::rgb(255, 255, 0));
                nvg.stroke_width(15.0);
                nvg.begin_path();
                nvg.move_to(-win_w * 0.2, 0.0);
                nvg.line_to(-win_w * 0.05, 0.0);
                nvg.arc(0.0, 0.0, win_w * 0.05, PI, 0.0, Winding::Ccw);
                nvg.line_to(win_w * 0.2, 0.0);
                nvg.stroke();

                Shape::circle(0.0, 0.0, win_w * 0.01)
                    .fill(Color::rgb(255, 255, 0))
                    .draw(nvg);
            });
        });
        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        self.nvg = None; // drop the NVG context to free resources
        true
    }
}

msfs::export_gauge!(
    name = attitude_gauge,
    state = AttitudeGauge,
    ctor = AttitudeGauge::new()
);

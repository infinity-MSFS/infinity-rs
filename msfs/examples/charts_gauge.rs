//! # Example: Approach chart viewer gauge
//!
//! Walks the full Charts API in three async stages:
//!   1. `init`            → request the chart index for KSEA via `FAA`
//!   2. index callback    → pick the first chart, request its pages
//!   3. pages callback    → request the first page's image
//!   4. image callback    → store the host image id, painted each frame
//!
//! Inter-stage state is held in a `Rc<RefCell<...>>` so the closures can hand
//! work back to the gauge without a 'static borrow on `&mut self`.

use infinity_rs::charts;
use infinity_rs::nvg::*;
use infinity_rs::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

const AIRPORT: &str = "KSEA";
const PROVIDER: &str = "FAA"; // or LIDO

#[derive(Default)]
struct ChartState {
    /// Most recent error surfaced by any stage of the pipeline.
    last_error: Option<ChartError>,
    /// Human-readable name of the chart we picked, for the on-screen label.
    chart_name: Option<String>,
    /// Loaded page image. Populated by the final callback.
    image: Option<ChartImage>,
}

pub struct ChartsGauge {
    nvg: Option<NvgContext>,
    font: Option<i32>,
    state: Rc<RefCell<ChartState>>,
    requested: bool,
}

impl ChartsGauge {
    pub fn new() -> Self {
        Self {
            nvg: None,
            font: None,
            state: Rc::new(RefCell::new(ChartState::default())),
            requested: false,
        }
    }

    /// Kick off the index → pages → image request chain.
    fn request_chart(&mut self, ctx: &Context) {
        let icao = charts::make_icao('A', "K1", AIRPORT, AIRPORT);
        let state_for_index = Rc::clone(&self.state);
        // `Context` is `Copy` — propagate it into each closure so deeper
        // stages can issue `get_page_image` without any raw-handle plumbing.
        let ctx = *ctx;

        let index_cb = move |result: ChartResult<ChartIndex>| match result {
            Err(e) => state_for_index.borrow_mut().last_error = Some(e),
            Ok(index) => {
                let Some(meta) = index.iter_categories().flat_map(|c| c.iter_charts()).next()
                else {
                    state_for_index.borrow_mut().last_error = Some(ChartError::NotFound);
                    return;
                };

                state_for_index.borrow_mut().chart_name = Some(meta.name().to_owned());
                let guid = meta.guid().to_owned();
                let state_for_pages = Rc::clone(&state_for_index);

                let pages_cb = move |result: ChartResult<ChartPages>| match result {
                    Err(e) => state_for_pages.borrow_mut().last_error = Some(e),
                    Ok(pages) => {
                        // First page, first url. Real gauges will iterate
                        // and prefer a known image type via `name`.
                        let Some((_, url)) =
                            pages.iter_pages().next().and_then(|p| p.iter_urls().next())
                        else {
                            state_for_pages.borrow_mut().last_error = Some(ChartError::NotFound);
                            return;
                        };

                        let url = url.to_owned();
                        let state_for_image = Rc::clone(&state_for_pages);

                        let image_cb = move |result: ChartResult<ChartImage>| {
                            let mut state = state_for_image.borrow_mut();
                            match result {
                                Ok(img) => state.image = Some(img),
                                Err(e) => state.last_error = Some(e),
                            }
                        };

                        if let Err(e) = charts::get_page_image(&ctx, &url, image_cb) {
                            state_for_pages.borrow_mut().last_error = Some(e);
                        }
                    }
                };

                if let Err(e) = charts::get_pages(&guid, pages_cb) {
                    state_for_index.borrow_mut().last_error = Some(e);
                }
            }
        };

        if let Err(e) = charts::get_index(icao, PROVIDER, index_cb) {
            self.state.borrow_mut().last_error = Some(e);
        }
    }
}

impl Default for ChartsGauge {
    fn default() -> Self {
        Self::new()
    }
}

impl Gauge for ChartsGauge {
    fn init(&mut self, ctx: &Context, _install: &mut GaugeInstall) -> bool {
        let nvg = match NvgContext::new(ctx) {
            Some(n) => n,
            None => return false,
        };
        self.font = nvg.create_font("sans", "./data/Roboto-Regular.ttf");
        self.nvg = Some(nvg);
        true
    }

    fn update(&mut self, ctx: &Context, _dt: f32) -> bool {
        // Defer the first request to `update` so `Context` is fully usable
        // and the host has finished its own gauge bring-up.
        if !self.requested {
            self.request_chart(ctx);
            self.requested = true;
        }
        true
    }

    fn draw(&mut self, _ctx: &Context, draw: &mut GaugeDraw) -> bool {
        let nvg = match &self.nvg {
            Some(n) => n,
            None => return false,
        };

        let win_w = draw.winWidth as f32;
        let win_h = draw.winHeight as f32;
        let px_ratio = draw.fbWidth as f32 / win_w;

        let state = self.state.borrow();
        let name = state.chart_name.as_deref();
        let error = state.last_error;
        let image = state.image.as_ref();

        nvg.frame(win_w, win_h, px_ratio, |nvg| {
            // Dark gauge background.
            Shape::rect(0.0, 0.0, win_w, win_h)
                .fill(Color::rgb(20, 20, 24))
                .draw(nvg);

            if let Some(image) = image {
                // Fit the chart inside the gauge while preserving aspect.
                let (iw, ih) = image.size(nvg);
                let (iw, ih) = (iw.max(1) as f32, ih.max(1) as f32);
                let scale = (win_w / iw).min(win_h / ih);
                let dw = iw * scale;
                let dh = ih * scale;
                let dx = (win_w - dw) * 0.5;
                let dy = (win_h - dh) * 0.5;

                let pattern = image.nvg_pattern(nvg, dx, dy, dw, dh, 0.0, 1.0);
                nvg.begin_path();
                nvg.rect(dx, dy, dw, dh);
                nvg.fill_paint(pattern);
                nvg.fill();
            } else if self.font.is_some() {
                // Status text while the request chain is in flight or has
                // failed at any stage.
                nvg.font_face("sans");
                nvg.font_size(28.0);
                nvg.text_align(Align::CENTER | Align::MIDDLE);
                nvg.fill_color(Color::rgb(220, 220, 220));
                let msg = match error {
                    Some(e) => format!("{AIRPORT} chart error: {e}"),
                    None => format!("Loading {AIRPORT} charts..."),
                };
                nvg.text(win_w * 0.5, win_h * 0.5, &msg);
            }

            // Top-left label with the chart name once it's known.
            if let (Some(name), true) = (name, self.font.is_some()) {
                nvg.font_face("sans");
                nvg.font_size(20.0);
                nvg.text_align(Align::LEFT | Align::TOP);
                nvg.fill_color(Color::rgb(255, 200, 0));
                nvg.text(10.0, 10.0, &format!("{AIRPORT} — {name}"));
            }
        });
        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        // Hand the chart image back to NVG before the context is dropped,
        // otherwise the texture leaks for the lifetime of the host process.
        if let Some(nvg) = &self.nvg {
            if let Some(img) = self.state.borrow_mut().image.take() {
                img.delete_with_nvg(nvg);
            }
        }
        self.nvg = None;
        true
    }
}

infinity_rs::export_gauge!(
    name = charts_gauge,
    state = ChartsGauge,
    ctor = ChartsGauge::new()
);

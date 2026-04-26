//! # Example: Weather Radar gauge backed by `MapView`
//!
//! Creates a [`MapView`] sized to the gauge's framebuffer, configures it as a
//! top-down weather radar with a rain-rate color ramp, and paints it into the
//! gauge each frame using NanoVG's image pattern API.
//!
//! Lifecycle mirrors `nvg_render.rs`:
//!   init  → MapView::new + NvgContext::new
//!   draw  → set follow / lat-long, build image pattern, fill rect
//!   kill  → both handles dropped (RAII deletes the texture / context)

use infinity_rs::map_view::*;
use infinity_rs::nvg::*;
use infinity_rs::prelude::*;

/// Texture resolution for the radar image. Chosen as a power of two so the
/// host can mipmap cleanly; resized lazily if the gauge framebuffer changes.
const RADAR_TEXTURE_SIZE: u32 = 512;

/// Default range of the radar sweep, in meters (≈ 50 NM).
const RADAR_RANGE_METERS: f32 = 92_600.0;

/// Half-cone of the radar beam, in radians (≈ 60° total).
const RADAR_CONE_HALF_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_6;

pub struct WeatherRadarGauge {
    nvg: Option<NvgContext>,
    map: Option<MapView>,
    texture_size: u32,
    font: Option<i32>,

    lat_var: AVar,
    lon_var: AVar,
}

impl WeatherRadarGauge {
    pub fn new() -> Self {
        Self {
            nvg: None,
            map: None,
            texture_size: RADAR_TEXTURE_SIZE,
            font: None,
            lat_var: AVar::new("PLANE LATITUDE", "DEGREES")
                .expect("Failed to create latitude AVar"),
            lon_var: AVar::new("PLANE LONGITUDE", "DEGREES")
                .expect("Failed to create longitude AVar"),
        }
    }

    /// Rain-rate ramp in mm/h, mapped from transparent → green → yellow → red.
    /// Matches the typical NEXRAD-style palette many GA EFIS units use.
    fn rain_palette() -> [RainRateColor; 5] {
        [
            RainRateColor { color: Color::TRANSPARENT,             rain_rate: 0.0 },
            RainRateColor { color: Color::rgb(  0, 200,   0),      rain_rate: 2.0 },
            RainRateColor { color: Color::rgb(255, 230,   0),      rain_rate: 12.0 },
            RainRateColor { color: Color::rgb(255, 120,   0),      rain_rate: 25.0 },
            RainRateColor { color: Color::rgb(220,   0,   0),      rain_rate: 50.0 },
        ]
    }
}

impl Default for WeatherRadarGauge {
    fn default() -> Self {
        Self::new()
    }
}

impl Gauge for WeatherRadarGauge {
    fn init(&mut self, ctx: &Context, _install: &mut GaugeInstall) -> bool {
        let nvg = match NvgContext::new(ctx) {
            Some(n) => n,
            None => return false,
        };

        let map = match MapView::new(
            ctx,
            self.texture_size,
            self.texture_size,
            RenderImageFlags::NONE,
        ) {
            Some(m) => m,
            None => return false,
        };

        // Top-down radar centered on the aircraft, isolines off so the
        // weather layer reads cleanly against the background fill.
        map.set_view_mode(ViewMode::Aerial);
        map.set_background_color(Color::BLACK);
        map.set_map_isolines_visibility(false);
        map.set_2d_view_follow_mode(true);
        map.set_2d_view_radius_meters(RADAR_RANGE_METERS);

        map.set_weather_radar_visibility(true);
        map.set_weather_radar_mode(WeatherRadarMode::TopView);
        map.set_weather_radar_cone_angle_radians(RADAR_CONE_HALF_ANGLE_RAD);
        map.set_weather_radar_rain_colors(&Self::rain_palette());

        map.set_visibility(true);

        self.font = nvg.create_font("sans", "./data/Roboto-Regular.ttf");
        self.nvg = Some(nvg);
        self.map = Some(map);
        true
    }

    fn update(&mut self, _ctx: &Context, _dt: f32) -> bool {
        true
    }

    fn draw(&mut self, _ctx: &Context, draw: &mut GaugeDraw) -> bool {
        let nvg = match &self.nvg {
            Some(n) => n,
            None => return false,
        };
        let map = match &self.map {
            Some(m) => m,
            None => return false,
        };

        let win_w = draw.winWidth as f32;
        let win_h = draw.winHeight as f32;
        let px_ratio = draw.fbWidth as f32 / win_w;

        // Follow mode handles the aircraft position automatically, but
        // pushing the current lat/long keeps the view tight when teleporting
        // (slew, replay seek) where follow can lag a frame.
        let lat = self.lat_var.get().unwrap_or(0.0);
        let lon = self.lon_var.get().unwrap_or(0.0);
        map.set_2d_view_lat_long(lat, lon);

        nvg.frame(win_w, win_h, px_ratio, |nvg| {
            // Square radar viewport, centered, leaving room for a label.
            let radar_size = win_w.min(win_h) * 0.95;
            let x = (win_w - radar_size) * 0.5;
            let y = (win_h - radar_size) * 0.5;

            // Black background under the radar so partial rain coverage
            // doesn't leak the gauge background through.
            Shape::rect(x, y, radar_size, radar_size)
                .fill(Color::BLACK)
                .draw(nvg);

            // Sample the map view texture across the radar viewport.
            let pattern = map.image_pattern(nvg, x, y, radar_size, radar_size, 0.0, 1.0);
            nvg.begin_path();
            nvg.rect(x, y, radar_size, radar_size);
            nvg.fill_paint(pattern);
            nvg.fill();

            // Range ring + crosshair overlay drawn on top of the radar texture.
            let cx = x + radar_size * 0.5;
            let cy = y + radar_size * 0.5;
            nvg.stroke_color(Color::rgb(0, 255, 128));
            nvg.stroke_width(1.5);
            nvg.begin_path();
            nvg.circle(cx, cy, radar_size * 0.5);
            nvg.circle(cx, cy, radar_size * 0.25);
            nvg.move_to(cx, y);
            nvg.line_to(cx, y + radar_size);
            nvg.move_to(x, cy);
            nvg.line_to(x + radar_size, cy);
            nvg.stroke();

            if self.font.is_some() {
                nvg.font_face("sans");
                nvg.font_size(radar_size * 0.06);
                nvg.fill_color(Color::rgb(0, 255, 128));
                nvg.text_align(Align::LEFT | Align::TOP);
                let nm = (RADAR_RANGE_METERS / 1852.0).round() as i32;
                nvg.text(x + 8.0, y + 8.0, &format!("WX {nm} NM"));
            }
        });
        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        // Drop order: NVG first (it may hold paints referencing the texture),
        // then the MapView which calls `fsMapViewDelete`.
        self.nvg = None;
        self.map = None;
        true
    }
}

infinity_rs::export_gauge!(
    name = weather_radar_gauge,
    state = WeatherRadarGauge,
    ctor = WeatherRadarGauge::new()
);

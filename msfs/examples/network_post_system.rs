// This example demonstrates a click handler triggering a POST request, and then the comm bus receiving the response and broadcasting it to any gauges that want it.
// A Lvar is set when request is pending and cleared when the response is received.

use msfs::prelude::*;

const EVT_POST_RESULT: &str = "infinity.demo/telemetry_post_result";

pub struct TelemetryGauge {
    l_pending: LVar,
}

impl TelemetryGauge {
    pub fn new() -> Self {
        let l_pending =
            LVar::new("L:INFINITY_TELEMETRY_PENDING", "Bool").expect("Failed to create LVar");
        Self { l_pending }
    }

    fn post_blob(&mut self) {
        let _ = self.l_pending.set(1.0);

        let blob = b"hello from wasm gauge".to_vec();

        let params = HttpParams {
            headers: vec![
                "Content-Type: application/octet-stream".to_string(),
                "User-Agent: InfinityDemo/1.0".to_string(),
            ],
            post_field: None,
            body: blob,
        };

        // We currently cannot pass var ownership, so we create a new one for the callback to use. This has no effect on the gauge since it's just a handle to an LVar with a known name.
        let mut l_for_cb = LVar::new("L:INFINITY_TELEMETRY_PENDING", "Bool")
            .expect("Failed to create LVar for callback");

        let _ = http_request(
            Method::Post,
            "https://example.com/telemetry",
            params,
            move |resp| {
                let _ = l_for_cb.set(0.0);

                let _ = commbus_call(
                    EVT_POST_RESULT,
                    &resp.data,
                    BroadcastFlags::JS | BroadcastFlags::WASM,
                );
            },
        );
    }
}

impl Gauge for TelemetryGauge {
    fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool {
        let _ = self.l_pending.set(0.0);
        true
    }

    fn update(&mut self, ctx: &Context, dt: f32) -> bool {
        true
    }

    fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool {
        true
    }

    fn kill(&mut self, ctx: &Context) -> bool {
        true
    }

    fn mouse(&mut self, _ctx: &Context, _x: f32, _y: f32, flags: i32) {
        if flags != 0 {
            self.post_blob();
        }
    }
}

msfs::export_gauge!(
    name = telemetry_gauge,
    state = TelemetryGauge,
    ctor = TelemetryGauge::new()
);

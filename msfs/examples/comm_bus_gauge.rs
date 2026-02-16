// This example shows how a user can click on a gauge and have it fire a comm bus event.

use msfs::prelude::*;

const EVT_TOGGLE: &str = "infinity.demo/toggle";

pub struct ToggleGauge {
    l_toggle: LVar,
    last_sent: i32,
}

impl ToggleGauge {
    pub fn new() -> Self {
        let l_toggle = LVar::new("L:INFINITY_TOGGLE", "Bool").expect("Failed to create LVar");
        Self {
            l_toggle,
            last_sent: 0,
        }
    }

    fn send(&self, v: i32) {
        let payload = v.to_le_bytes();
        let _ = commbus_call(
            EVT_TOGGLE,
            &payload,
            BroadcastFlags::JS | BroadcastFlags::WASM,
        );
    }
}

impl Gauge for ToggleGauge {
    fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool {
        let _ = self.l_toggle.set(0.0);
        true
    }

    fn update(&mut self, ctx: &Context, dt: f32) -> bool {
        let v = self.l_toggle.get().unwrap_or(0.0) as i32;
        if v != self.last_sent {
            self.send(v);
            self.last_sent = v;
        }
        true
    }

    fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool {
        // Actual gauge rendering happens here, This example doesn't cover rendering so we just draw a blank gauge.
        true
    }

    fn kill(&mut self, ctx: &Context) -> bool {
        true
    }

    fn mouse(&mut self, _ctx: &Context, _x: f32, _y: f32, _flags: i32) {
        if (flags == 0) {
            return;
        }

        let cur = self.l_toggle.get().unwrap_or(0.0);
        let next = if cur >= 0.5 { 0.0 } else { 1.0 };
        let _ = self.l_toggle.set(next);
    }
}

msfs::export_gauge!(
    name = toggle_gauge,
    state = ToggleGauge,
    ctor = ToggleGauge::new()
);

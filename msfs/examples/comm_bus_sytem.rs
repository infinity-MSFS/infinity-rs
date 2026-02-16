use msfs::prelude::*;

const EVT_CMD: &str = "infinity.demo/system_cmd";
const EVT_STATE: &str = "infinity.demo/system_state";

#[derive(Default)]
pub struct CommbusStateSystem {
    l_enabled: LVar,
    _sub_cmd: Subscription,
    accum: f32,
}

impl CommbusStateSystem {
    pub fn new() -> Self {
        let l_enabled = LVar::new("L:INFINITY_DEMO_ENABLED", "Bool").expect("LVar create failed");

        // Subscribe to a command bus:
        // payload[0] = 0 -> disable
        // payload[0] = 1 -> enable
        // payload[0] = 2 -> toggle
        let mut l_for_cb =
            LVar::new("L:INFINITY_DEMO_ENABLED", "Bool").expect("LVar create failed");

        let sub = Subscription::subscribe(EVT_CMD, move |bytes| {
            let cmd = bytes.get(0).copied().unwrap_or(0);
            let cur = l_for_cb.get().unwrap_or(0.0);

            let next = match cmd {
                0 => 0.0,
                1 => 1.0,
                2 => {
                    if cur >= 0.5 {
                        0.0
                    } else {
                        1.0
                    }
                }
                _ => cur,
            };

            let _ = l_for_cb.set(next);
        })
        .expect("commbus subscribe failed");

        Self {
            l_enabled,
            _sub_cmd: sub,
            accum: 0.0,
        }
    }

    fn broadcast_state(&self) {
        let enabled_u8 = (self.l_enabled.get().unwrap_or(0.0) >= 0.5) as u8;
        let payload = [enabled_u8];

        let _ = commbus_call(EVT_STATE, &payload, BroadcastFlags::ALL);
    }
}

impl System for CommbusStateSystem {
    fn init(&mut self, ctx: &Context, install: &SystemInstall) -> bool {
        let _ = self.l_enabled.set(0.0);
        self.broadcast_state();
        true
    }

    fn update(&mut self, ctx: &Context, dt: f32) -> bool {
        self.accum += dt;

        if self.accum >= 0.5 {
            self.accum = 0.0;
            self.broadcast_state();
        }
        true
    }

    fn kill(&mut self, ctx: &Context) -> bool {
        true
    }
}

msfs::export_system!(
    name = commbus_state_system,
    state = CommbusStateSystem,
    ctor = CommbusStateSystem::new()
);

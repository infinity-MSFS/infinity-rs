//! # Example: contrail VFX bound to the user aircraft
//!
//! Spawns a particle effect on the player's sim object during `init`, plays
//! it once it's valid, and toggles emission based on engine N1 each tick. The
//! effect is destroyed automatically when the system is torn down — `Drop`
//! on [`VfxInstance`] calls `fsVfxDestroyInstance` for us.
//!
//! Replace the GUID with one from your package's `effects/` folder.

use infinity_rs::prelude::*;

const VFX_GUID: &str = "{00000000-0000-0000-0000-000000000000}";
const NODE_NAME: &str = "FUSELAGE";

/// N1 above which we consider the engine "running" and want emission.
const N1_ON_THRESHOLD: f64 = 25.0;

pub struct ContrailSystem {
    vfx: Option<VfxInstance>,
    n1_var: AVar,
    user_aircraft_var: AVar,
    started: bool,
}

impl ContrailSystem {
    pub fn new() -> Self {
        Self {
            vfx: None,
            n1_var: AVar::new("ENG N1 RPM:1", "PERCENT")
                .expect("Failed to create N1 AVar"),
            // Sim-object id of the user aircraft. Returned as a double; we
            // truncate to the host `SimObjId` width.
            user_aircraft_var: AVar::new("USER AIRCRAFT SIM OBJ ID", "NUMBER")
                .expect("Failed to create user-object AVar"),
            started: false,
        }
    }
}

impl Default for ContrailSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for ContrailSystem {
    fn init(&mut self, _ctx: &Context, _install: &SystemInstall) -> bool {
        // Drive the smoke density from N1 via an RPN expression evaluated
        // by the host every frame. Demonstrates `VfxParam` plumbing.
        let params = [VfxParam::new(
            "Density",
            "(A:ENG N1 RPM:1, percent) 100 / 0.0 max 1.0 min",
        )];

        let sim_obj = self.user_aircraft_var.get().unwrap_or(0.0) as SimObjId;

        self.vfx = VfxInstance::spawn_on_sim_object(
            VFX_GUID,
            sim_obj,
            Some(NODE_NAME),
            vec3d(0.0, 0.0, -3.0), // 3m aft of the node
            vec3d(0.0, 0.0, 0.0),
            None,
            &params,
        );
        true
    }

    fn update(&mut self, _ctx: &Context, _dt: f32) -> bool {
        let Some(vfx) = &self.vfx else { return true };

        // The host may invalidate the instance independently (flight reload,
        // teleport, etc.) — drop it so a future init can respawn cleanly.
        if !vfx.is_valid() {
            self.vfx = None;
            self.started = false;
            return true;
        }

        if !self.started {
            vfx.play();
            self.started = true;
        }

        let n1 = self.n1_var.get().unwrap_or(0.0);
        match (n1 >= N1_ON_THRESHOLD, vfx.is_playing()) {
            (true, false) => {
                vfx.play();
            }
            (false, true) if vfx.is_min_time_passed() => {
                vfx.stop();
            }
            _ => {}
        }
        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        // Drop releases via `fsVfxDestroyInstance`.
        self.vfx = None;
        true
    }
}

infinity_rs::export_system!(
    name = contrail_system,
    state = ContrailSystem,
    ctor = ContrailSystem::new()
);

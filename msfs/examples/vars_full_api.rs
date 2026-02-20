//! This example demonstrates a variety of ways to interact with the vars API, including:
//! - Creating `AVar` and `LVar`
//! - Reading (default target + explicit target)
//! - Indexed AVars (param array with an index)
//! - Low-level `get_with(param, target)`
//! - Writing to LVars (`set`, `set_target`)
//! - Struct sugar via `#[derive(VarStruct)]`

use msfs::prelude::*;
use msfs::{
    sys::{FS_OBJECT_ID_USER_AIRCRAFT, FS_OBJECT_ID_USER_CURRENT},
    vars::{AVar, VarParamArray1, empty_param_array},
};
use msfs_derive::VarStruct;

// Control LVars (inputs)
const L_ENABLED: &str = "L:INFINITY_VARS_DEMO_ENABLED";
const L_DO_WRITE: &str = "L:INFINITY_VARS_DEMO_DO_WRITE";

// Output LVars (results you can observe in-sim)
const L_OUT_AIRSPEED: &str = "L:INFINITY_VARS_DEMO_AIRSPEED_KTS";
const L_OUT_AIRSPEED_CUR: &str = "L:INFINITY_VARS_DEMO_AIRSPEED_KTS_CURRENT";
const L_OUT_ENG1_RPM: &str = "L:INFINITY_VARS_DEMO_ENG1_RPM";
const L_OUT_ENG2_RPM: &str = "L:INFINITY_VARS_DEMO_ENG2_RPM";
const L_OUT_ENG1_RPM_LOW: &str = "L:INFINITY_VARS_DEMO_ENG1_RPM_LOWLEVEL";
const L_OUT_CUSTOM: &str = "L:INFINITY_VARS_DEMO_CUSTOM_VALUE";
const L_OUT_SNAPSHOT_ALT: &str = "L:INFINITY_VARS_DEMO_SNAPSHOT_ALT_FT";
const L_OUT_SNAPSHOT_HDG: &str = "L:INFINITY_VARS_DEMO_SNAPSHOT_HDG_DEG";

#[derive(Debug, Clone, Copy, VarStruct)]
struct Snapshot {
    #[var(name = "A:PLANE ALTITUDE", unit = "Feet", kind = "A")]
    altitude_ft: f64,

    #[var(name = "A:PLANE HEADING DEGREES TRUE", unit = "Degrees", kind = "A")]
    heading_deg_true: f64,

    // Demonstrates indexed AVar in VarStruct
    #[var(name = "A:GENERAL ENG RPM", unit = "RPM", kind = "A", index = 1)]
    eng1_rpm: f64,

    // Demonstrates LVar in VarStruct
    #[var(
        name = "L:INFINITY_VARS_DEMO_CUSTOM_VALUE",
        unit = "Number",
        kind = "L"
    )]
    custom_value: f64,
}

pub struct VarsFullApiSystem {
    // Inputs
    l_enabled: LVar,
    l_do_write: LVar,

    // Outputs
    l_out_airspeed: LVar,
    l_out_airspeed_current: LVar,
    l_out_eng1_rpm: LVar,
    l_out_eng2_rpm: LVar,
    l_out_eng1_rpm_low: LVar,
    l_out_custom: LVar,
    l_out_snapshot_alt: LVar,
    l_out_snapshot_hdg: LVar,

    // Vars we read from the sim
    a_airspeed: AVar,
    a_eng_rpm: AVar,

    accum: f32,
}

impl VarsFullApiSystem {
    pub fn new() -> Self {
        // Control switches
        let l_enabled = LVar::new(L_ENABLED, "Bool").expect("LVar create failed");
        let l_do_write = LVar::new(L_DO_WRITE, "Bool").expect("LVar create failed");

        // Output channels
        let l_out_airspeed = LVar::new(L_OUT_AIRSPEED, "Number").expect("LVar create failed");
        let l_out_airspeed_current =
            LVar::new(L_OUT_AIRSPEED_CUR, "Number").expect("LVar create failed");
        let l_out_eng1_rpm = LVar::new(L_OUT_ENG1_RPM, "Number").expect("LVar create failed");
        let l_out_eng2_rpm = LVar::new(L_OUT_ENG2_RPM, "Number").expect("LVar create failed");
        let l_out_eng1_rpm_low =
            LVar::new(L_OUT_ENG1_RPM_LOW, "Number").expect("LVar create failed");
        let l_out_custom = LVar::new(L_OUT_CUSTOM, "Number").expect("LVar create failed");
        let l_out_snapshot_alt =
            LVar::new(L_OUT_SNAPSHOT_ALT, "Number").expect("LVar create failed");
        let l_out_snapshot_hdg =
            LVar::new(L_OUT_SNAPSHOT_HDG, "Number").expect("LVar create failed");

        // The sim vars (AVars)
        let a_airspeed = AVar::new("A:AIRSPEED INDICATED", "Knots").expect("AVar create failed");
        let a_eng_rpm = AVar::new("A:GENERAL ENG RPM", "RPM").expect("AVar create failed");

        Self {
            l_enabled,
            l_do_write,
            l_out_airspeed,
            l_out_airspeed_current,
            l_out_eng1_rpm,
            l_out_eng2_rpm,
            l_out_eng1_rpm_low,
            l_out_custom,
            l_out_snapshot_alt,
            l_out_snapshot_hdg,
            a_airspeed,
            a_eng_rpm,
            accum: 0.0,
        }
    }

    fn tick(&mut self) {
        // 1) Read a plain AVar (default target)
        if let Ok(v) = self.a_airspeed.get() {
            let _ = self.l_out_airspeed.set(v);
        }

        // 2) Read that same AVar from an explicit target
        if let Ok(v) = self.a_airspeed.get_target(FS_OBJECT_ID_USER_CURRENT) {
            let _ = self.l_out_airspeed_current.set(v);
        }

        // 3) Indexed AVar reads (engine 1 + engine 2)
        if let Ok(v) = self.a_eng_rpm.get_indexed(1) {
            let _ = self.l_out_eng1_rpm.set(v);
        }
        if let Ok(v) = self
            .a_eng_rpm
            .get_indexed_target(2, FS_OBJECT_ID_USER_AIRCRAFT)
        {
            let _ = self.l_out_eng2_rpm.set(v);
        }

        // 4) Low-level equivalent: `get_with(param, target)`
        // Here we build a one-element param array containing the index.
        let mut param = VarParamArray1::index(1);
        if let Ok(v) = self
            .a_eng_rpm
            .get_with(param.as_raw_mut(), FS_OBJECT_ID_USER_AIRCRAFT)
        {
            let _ = self.l_out_eng1_rpm_low.set(v);
        }

        // 5) Writing values (safe demo: write to an LVar)
        // Flip `L:INFINITY_VARS_DEMO_DO_WRITE` to 1 to make it write a changing value.
        let do_write = self.l_do_write.get().unwrap_or(0.0) >= 0.5;
        if do_write {
            let next = self.l_out_custom.get().unwrap_or(0.0) + 1.0;
            let _ = self.l_out_custom.set(next);
            // Also demonstrates the API shape; for LVars the target is ignored.
            let _ = self
                .l_out_custom
                .set_target(FS_OBJECT_ID_USER_CURRENT, next);
        }

        // 6) Struct sugar: one call to fetch multiple vars
        if let Ok(s) = Snapshot::get() {
            let _ = self.l_out_snapshot_alt.set(s.altitude_ft);
            let _ = self.l_out_snapshot_hdg.set(s.heading_deg_true);
            // And you can push everything back out with `s.set()` if you want.
            let _ = s.set();
        }

        // 7) AVar “no params” low-level call
        // This is mostly here to show the signature; it should behave like `get_target`.
        let _ = self
            .a_airspeed
            .get_with(empty_param_array(), FS_OBJECT_ID_USER_AIRCRAFT);
    }
}

impl System for VarsFullApiSystem {
    fn init(&mut self, _ctx: &Context, _install: &SystemInstall) -> bool {
        let _ = self.l_enabled.set(1.0);
        let _ = self.l_do_write.set(0.0);

        // Clear outputs so you can see updates.
        let _ = self.l_out_airspeed.set(0.0);
        let _ = self.l_out_airspeed_current.set(0.0);
        let _ = self.l_out_eng1_rpm.set(0.0);
        let _ = self.l_out_eng2_rpm.set(0.0);
        let _ = self.l_out_eng1_rpm_low.set(0.0);
        let _ = self.l_out_custom.set(0.0);
        let _ = self.l_out_snapshot_alt.set(0.0);
        let _ = self.l_out_snapshot_hdg.set(0.0);
        true
    }

    fn update(&mut self, _ctx: &Context, dt: f32) -> bool {
        self.accum += dt;

        // Run at 2 Hz to keep the demo lightweight.
        if self.accum >= 0.5 {
            self.accum = 0.0;
            let enabled = self.l_enabled.get().unwrap_or(0.0) >= 0.5;
            if enabled {
                self.tick();
            }
        }

        true
    }

    fn kill(&mut self, _ctx: &Context) -> bool {
        true
    }
}

msfs::export_system!(
    name = vars_full_api,
    state = VarsFullApiSystem,
    ctor = VarsFullApiSystem::new()
);

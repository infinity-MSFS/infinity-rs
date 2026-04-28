//! WASM gauge example: opens SimConnect from inside a panel.cfg-mounted gauge
//! and mirrors the user aircraft's altitude into an LVar (`L:INFINITY_SC_ALT`).
//!
//! Build with:
//!     cargo build -p infinity-rs --target wasm32-wasip1 \
//!         --features simconnect --example simconnect_gauge --release
//!
//! SimConnect symbols are resolved by the MSFS runtime at load time, so no
//! additional link configuration is required for the WASM target.

use infinity_rs::prelude::*;
use infinity_rs::simconnect::{
    DataDefinitionId, DataRequestFlag, DataRequestId, DataType, ObjectId, Period, Recv,
    SimConnect, SimObjectDataView, UNUSED,
};

#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
struct Sample {
    altitude_ft: f64,
}

const DEF: DataDefinitionId = DataDefinitionId::new(1);
const REQ: DataRequestId = DataRequestId::new(1);

pub struct SimConnectGauge {
    sc: Option<SimConnect>,
    l_alt: LVar,
}

impl SimConnectGauge {
    pub fn new() -> Self {
        Self {
            sc: None,
            l_alt: LVar::new("L:INFINITY_SC_ALT", "feet").expect("LVar create failed"),
        }
    }

    fn open(&mut self) {
        let mut sc = match SimConnect::open("infinity_sc_gauge") {
            Ok(s) => s,
            Err(_) => return,
        };

        let _ = sc.add_to_data_definition(
            DEF,
            "PLANE ALTITUDE",
            "feet",
            DataType::SIMCONNECT_DATATYPE_FLOAT64,
            0.0,
            UNUSED,
        );

        let _ = sc.request_data_on_sim_object(
            REQ,
            DEF,
            ObjectId::USER,
            Period::SIMCONNECT_PERIOD_SIM_FRAME,
            DataRequestFlag::CHANGED,
            0,
            0,
            0,
        );

        self.sc = Some(sc);
    }
}

impl Default for SimConnectGauge {
    fn default() -> Self { Self::new() }
}

impl Gauge for SimConnectGauge {
    fn init(&mut self, _ctx: &Context, _install: &mut GaugeInstall) -> bool {
        self.open();
        true
    }

    fn update(&mut self, _ctx: &Context, _dt: f32) -> bool {
        let Some(sc) = self.sc.as_mut() else { return true };
        let l_alt = &mut self.l_alt;

        let _ = sc.dispatch(|msg| {
            if let Recv::SimObjectData(data) = msg {
                let view = SimObjectDataView(data);
                if view.dwRequestID() == REQ.raw() {
                    let s: Sample = unsafe { view.data_as() };
                    let _ = l_alt.set(s.altitude_ft);
                }
            }
        });
        true
    }

    fn draw(&mut self, _ctx: &Context, _draw: &mut GaugeDraw) -> bool { true }

    fn kill(&mut self, _ctx: &Context) -> bool {
        self.sc = None;
        true
    }
}

infinity_rs::export_gauge!(
    name = simconnect_gauge,
    state = SimConnectGauge,
    ctor = SimConnectGauge::new()
);

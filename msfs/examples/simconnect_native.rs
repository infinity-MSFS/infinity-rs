//! Native SimConnect example: connects to a running MSFS instance and prints
//! the user aircraft's altitude / airspeed / heading every sim frame.
//!
//! Build & run with:
//!     cargo run -p infinity-rs --features simconnect --example simconnect_native
//!
//! MSFS must already be running with a flight loaded.

use std::ffi::CStr;
use std::{thread, time::Duration};

// `infinity-rs` is primarily compiled as a WASM gauge cdylib whose Drop impls
// reference symbols provided by the MSFS WASM runtime (`fsCommBus*`, `fsIO*`,
// `fsEvents*`, `nvg*`, …). These don't exist on a native host, so we stub them
// as no-ops here just to satisfy the linker — they are never actually called by
// the SimConnect path used in this example.
#[allow(non_snake_case)]
mod wasm_runtime_stubs {
    macro_rules! stub {
        ($($name:ident),* $(,)?) => {
            $(
                #[unsafe(no_mangle)]
                pub extern "C" fn $name() {}
            )*
        };
    }
    stub!(
        fsChartsFreeChartIndex, fsChartsFreeChartPages,
        fsCommBusCall, fsCommBusUnregisterOneEvent,
        fsEventsUnregisterKeyEventHandler,
        fsFlowUnregister, fsFlowUnregisterAll,
        fsIOClose, fsIOGetFileSize, fsIOGetLastError, fsIOHasError,
        fsIOIsDone, fsIOIsOpened, fsIOOpen, fsIOWrite,
        fsMapViewDelete,
        fsNetworkHttpRequestGetData, fsNetworkHttpRequestGetDataSize,
        fsPlannedRouteGetEfbRoute, fsPlannedRouteRespondToRequest,
        fsPlannedRouteUnregisterForBroadcast, fsPlannedRouteUnregisterForRequest,
        fsVarsGetUnitId,
        fsVfxDestroyInstance,
        nvgDeleteInternal,
    );
}

use infinity_rs::simconnect::{
    DataDefinitionId, DataRequestFlag, DataRequestId, DataType, ObjectId, Period, Recv, SimConnect,
    SimObjectDataView, UNUSED,
};

#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
struct AircraftData {
    altitude_ft: f64,
    airspeed_kt: f64,
    heading_deg: f64,
}

const DEF_AIRCRAFT: DataDefinitionId = DataDefinitionId::new(1);
const REQ_AIRCRAFT: DataRequestId = DataRequestId::new(1);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut sc = SimConnect::open("infinity-rs native demo")?;
    println!("Connected to SimConnect.");

    sc.add_to_data_definition(
        DEF_AIRCRAFT,
        "PLANE ALTITUDE",
        "feet",
        DataType::SIMCONNECT_DATATYPE_FLOAT64,
        0.0,
        UNUSED,
    )?;
    sc.add_to_data_definition(
        DEF_AIRCRAFT,
        "AIRSPEED INDICATED",
        "knots",
        DataType::SIMCONNECT_DATATYPE_FLOAT64,
        0.0,
        UNUSED,
    )?;
    sc.add_to_data_definition(
        DEF_AIRCRAFT,
        "PLANE HEADING DEGREES TRUE",
        "degrees",
        DataType::SIMCONNECT_DATATYPE_FLOAT64,
        0.0,
        UNUSED,
    )?;

    sc.request_data_on_sim_object(
        REQ_AIRCRAFT,
        DEF_AIRCRAFT,
        ObjectId::USER,
        Period::SIMCONNECT_PERIOD_SIM_FRAME,
        DataRequestFlag::CHANGED,
        0,
        0,
        0,
    )?;

    println!("Polling sim... (Ctrl+C to quit)");

    loop {
        sc.dispatch(|msg| match msg {
            Recv::SimObjectData(data) if SimObjectDataView(data).dwRequestID() == REQ_AIRCRAFT.raw() => {
                let v = SimObjectDataView(data);
                let payload: AircraftData = unsafe { v.data_as() };
                let alt = payload.altitude_ft;
                let ias = payload.airspeed_kt;
                let hdg = payload.heading_deg;
                println!("alt={alt:8.1} ft  ias={ias:6.1} kt  hdg={hdg:6.1}°");
            }
            Recv::Open(open) => {
                let name = unsafe { CStr::from_ptr(open.szApplicationName.as_ptr()) };
                println!("OPEN: {}", name.to_string_lossy());
            }
            Recv::Quit(_) => {
                println!("Sim quit.");
                std::process::exit(0);
            }
            Recv::Exception(ex) => {
                let view = infinity_rs::simconnect::ExceptionView(ex);
                eprintln!(
                    "EXCEPTION {:?} (sendID={}, index={})",
                    view.exception(),
                    view.dwSendID(),
                    view.dwIndex(),
                );
            }
            _ => {}
        })?;

        thread::sleep(Duration::from_millis(50));
    }
}

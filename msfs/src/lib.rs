mod comm_bus;
mod events;
pub mod io;
mod network;
pub mod sys;
mod utils;
pub mod vars;

// New: host API indirection for native testing, plus a native NanoVG backend.
#[cfg(not(target_arch = "wasm32"))]
pub mod host;

pub mod render;

use crate::sys::{FsContext, sGaugeDrawData, sGaugeInstallData, sSystemInstallData};
pub use msfs_derive::{AbiTypes, GaugeModule, SystemModule, export_gauge_abi, export_system_abi};

pub struct MsfsAbi;

impl AbiTypes for MsfsAbi {
    type FsContext = FsContext;
    type SystemInstallData = sSystemInstallData;
    type GaugeInstallData = sGaugeInstallData;
    type GaugeDrawData = sGaugeDrawData;
}

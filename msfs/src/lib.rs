mod comm_bus;
mod events;
pub mod io;
mod network;
mod sys;
mod utils;
pub mod vars;

use crate::sys::{FsContext, sGaugeDrawData, sGaugeInstallData, sSystemInstallData};
pub use msfs_derive::{AbiTypes, GaugeModule, SystemModule, export_gauge_abi, export_system_abi};

pub struct MsfsAbi;

impl AbiTypes for MsfsAbi {
    type FsContext = FsContext;
    type SystemInstallData = sSystemInstallData;
    type GaugeInstallData = sGaugeInstallData;
    type GaugeDrawData = sGaugeDrawData;
}

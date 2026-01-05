mod sys;
mod vars;

pub use msfs_derive::{AbiTypes, GaugeModule, SystemModule, export_gauge_abi, export_system_abi};
use crate::sys::{sGaugeDrawData, sGaugeInstallData, sSystemInstallData, FsContext};

pub struct MsfsAbi;

impl AbiTypes for MsfsAbi {
    type FsContext = FsContext;
    type SystemInstallData = sSystemInstallData;
    type GaugeInstallData = sGaugeInstallData;
    type GaugeDrawData = sGaugeDrawData;
}

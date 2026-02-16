use crate::sys::*;

pub struct Fs2024;

pub trait Abi {
    type Context;
    type SystemInstall;
    type GaugeInstall;
    type GaugeDraw;
}

impl Abi for Fs2024 {
    type Context = FsContext;
    type SystemInstall = sSystemInstallData;
    type GaugeInstall = sGaugeInstallData;
    type GaugeDraw = sGaugeDrawData;
}

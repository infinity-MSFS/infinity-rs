mod abi;
mod comm_bus;
mod context;
mod events;
mod exports;
pub mod io;
mod modules;
mod network;
pub mod prelude;
mod sys;
mod types;
mod utils;
mod vars;

// New: host API indirection for native testing, plus a native NanoVG backend.
#[cfg(not(target_arch = "wasm32"))]
pub mod host;

pub mod render;

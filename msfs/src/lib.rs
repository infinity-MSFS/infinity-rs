pub mod abi;
pub mod comm_bus;
pub mod context;
pub mod events;
pub mod exports;
pub mod io;
pub mod modules;
pub mod network;
pub mod prelude;
pub mod sys;
pub mod types;
pub mod utils;
pub mod vars;

// New: host API indirection for native testing, plus a native NanoVG backend.
#[cfg(not(target_arch = "wasm32"))]
pub mod host;

pub mod render;

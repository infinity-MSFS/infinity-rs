// Re-export used by macros so downstream crates don't need a direct `paste` dependency.
pub use paste as __paste;

pub mod abi;
pub mod charts;
pub mod comm_bus;
pub mod context;
pub mod events;
pub mod exports;
pub mod flow;
pub mod io;
pub mod map_view;
pub mod modules;
pub mod network;
pub mod planned_route;
pub mod prelude;
#[cfg(feature = "simconnect")]
pub mod simconnect;
pub mod sys;
pub mod types;
pub mod utils;
pub mod vars;
pub mod vfx;

// New: host API indirection for native testing, plus a native NanoVG backend.
#[cfg(not(target_arch = "wasm32"))]
pub mod host;

pub mod nvg;

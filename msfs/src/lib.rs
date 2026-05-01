// `sys` is bindgen output mirroring C identifiers (camelCase types,
// `dwFoo` fields, `Fs*_Fs*_*` enum constants). Allow them at the crate
// root rather than peppering every match arm and accessor with attrs.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

// Re-export used by macros so downstream crates don't need a direct `paste` dependency.
pub use paste as __paste;

#[cfg(feature = "derive")]
pub use infinity_rs_derive::*;

pub mod abi;
pub mod sys;
pub mod types;

#[cfg(feature = "simconnect")]
pub mod simconnect;

// Modules below call into the MSFS WASM host runtime (`nvg*`, `fsIO*`,
// `fsCommBus*`, `fsRender*`, …). They are gated behind the `wasm` feature
// (default-on) so native consumers — e.g. SimConnect-only tools that link
// against this crate — can opt out and avoid pulling unresolved externals.
#[cfg(feature = "wasm")]
pub mod charts;
#[cfg(feature = "wasm")]
pub mod comm_bus;
#[cfg(feature = "wasm")]
pub mod context;
#[cfg(feature = "wasm")]
pub mod events;
#[cfg(feature = "wasm")]
pub mod exports;
#[cfg(feature = "wasm")]
pub mod flow;
#[cfg(feature = "wasm")]
pub mod io;
#[cfg(feature = "wasm")]
pub mod map_view;
#[cfg(feature = "wasm")]
pub mod modules;
#[cfg(feature = "wasm")]
pub mod network;
#[cfg(feature = "wasm")]
pub mod nvg;
#[cfg(feature = "wasm")]
pub mod planned_route;
#[cfg(feature = "wasm")]
pub mod prelude;
#[cfg(feature = "wasm")]
pub mod utils;
#[cfg(feature = "wasm")]
pub mod vars;
#[cfg(feature = "wasm")]
pub mod vfx;

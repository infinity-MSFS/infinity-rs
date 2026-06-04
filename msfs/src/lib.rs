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

// Modules below call into the MSFS host runtime (`nvg*`, `fsIO*`,
// `fsCommBus*`, `fsRender*`, …). They are gated behind the `host-abi` feature
// (implied by `wasm`, default-on) so native consumers — e.g. a SimConnect-only
// tool — can opt out and avoid pulling the unresolved externals, while the
// emulator's native build modes (`native-dynamic` / `native-exe`) opt back in
// and resolve those symbols against a host shim instead of the MSFS loader.
#[cfg(feature = "host-abi")]
pub mod charts;
#[cfg(feature = "host-abi")]
pub mod comm_bus;
#[cfg(feature = "host-abi")]
pub mod context;
#[cfg(feature = "host-abi")]
pub mod events;
#[cfg(feature = "host-abi")]
pub mod exports;
#[cfg(feature = "host-abi")]
pub mod flow;
#[cfg(feature = "host-abi")]
pub mod io;
#[cfg(feature = "host-abi")]
pub mod map_view;
#[cfg(feature = "host-abi")]
pub mod modules;
#[cfg(feature = "host-abi")]
pub mod network;
#[cfg(feature = "host-abi")]
pub mod nvg;
#[cfg(feature = "host-abi")]
pub mod planned_route;
#[cfg(feature = "host-abi")]
pub mod prelude;
#[cfg(feature = "host-abi")]
pub mod utils;
#[cfg(feature = "host-abi")]
pub mod vars;
#[cfg(feature = "host-abi")]
pub mod vfx;

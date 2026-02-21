# infinity-rs

Safe, idiomatic Rust bindings for the **Microsoft Flight Simulator 2024 (MSFS 2024) WASM SDK**.

Write MSFS gauges and systems in Rust — with full access to SimVars, the Comm Bus, async HTTP, file I/O, and NanoVG rendering. More SDK APIs (events, weather, flight plan, charts, and more) are actively being bound — see the [Work in Progress](#work-in-progress) section.

---

## Workspace Layout

| Crate | Description |
|---|---|
| `msfs` | Main bindings crate — re-exports everything you need |
| `msfs_derive` | Proc-macro helpers (`#[derive(VarStruct)]`) |
| `msfs_sdk` | Build helper that locates the installed MSFS 2024 SDK |

---

## Prerequisites

| Requirement | Notes |
|---|---|
| **MSFS 2024 SDK** | Install via the MSFS Dev Mode or the standalone SDK installer |
| `MSFS2024_SDK` env var | Must point to your SDK root (e.g. `C:\MSFS 2024 SDK`) |
| Rust nightly / `wasm32-wasip1` target | `rustup target add wasm32-wasip1` |
| `clang` + `llvm-ar` | Required by the build script for WASM compilation |

The build script automatically detects whether you are targeting `wasm32` and adjusts compiler flags, NanoVG compilation, and linking accordingly. For native builds (testing / tooling) it links against the SimConnect SDK instead.

---

## Getting Started

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
msfs = { path = "path/to/infinity-rs/msfs" }
msfs_derive = { path = "path/to/infinity-rs/msfs_derive" }  # only needed for VarStruct
```

Set the SDK environment variable before building:

```powershell
$env:MSFS2024_SDK = "C:\MSFS 2024 SDK"
cargo build --target wasm32-wasip1
```

---

## Core Concepts

### Systems and Gauges

Everything in MSFS runs as either a **System** or a **Gauge**. Implement the corresponding trait and export it with the matching macro.

**System** — logic-only, no rendering:

```rust
use msfs::prelude::*;

pub struct MySystem { /* ... */ }

impl System for MySystem {
    fn init(&mut self, ctx: &Context, install: &SystemInstall) -> bool { true }
    fn update(&mut self, ctx: &Context, dt: f32) -> bool { true }
    fn kill(&mut self, ctx: &Context) -> bool { true }
}

msfs::export_system!(
    name  = my_system,
    state = MySystem,
    ctor  = MySystem::new(),
);
```

**Gauge** — rendered panel element with optional mouse input:

```rust
use msfs::prelude::*;

pub struct MyGauge { /* ... */ }

impl Gauge for MyGauge {
    fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool { true }
    fn update(&mut self, ctx: &Context, dt: f32) -> bool { true }
    fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool { true }
    fn kill(&mut self, ctx: &Context) -> bool { true }

    fn mouse(&mut self, _ctx: &Context, x: f32, y: f32, flags: i32) { /* optional */ }
}

msfs::export_gauge!(
    name  = my_gauge,
    state = MyGauge,
    ctor  = MyGauge::new(),
);
```

The macros emit the correctly named `extern "C"` entry points expected by the simulator.

---

## Features

### SimVars — `msfs::vars`

Read and write simulation variables via `AVar` (A-vars) and `LVar` (L-vars).

```rust
use msfs::vars::{AVar, LVar};

// A-var: read-only aircraft variable
let airspeed = AVar::new("AIRSPEED INDICATED", "Knots")?;
let kts: f64 = airspeed.get()?;

// L-var: readable and writable local variable
let mut flag = LVar::new("L:MY_GAUGE_ACTIVE", "Bool")?;
flag.set(1.0)?;
let val = flag.get()?;
```

**Indexed A-vars** (e.g. per-engine data):

```rust
use msfs::vars::{AVar, VarParamArray1};

let eng_rpm = AVar::new("GENERAL ENG RPM", "RPM")?;
let rpm = eng_rpm.get_with(VarParamArray1::new(1), Default::default())?; // engine 1
```

#### `#[derive(VarStruct)]`

Bundle multiple vars into a single struct and snapshot them all at once:

```rust
use msfs_derive::VarStruct;

#[derive(Debug, Clone, Copy, VarStruct)]
struct FlightData {
    #[var(name = "A:PLANE ALTITUDE",               unit = "Feet",    kind = "A")]
    altitude_ft: f64,

    #[var(name = "A:PLANE HEADING DEGREES TRUE",   unit = "Degrees", kind = "A")]
    heading_deg: f64,

    #[var(name = "A:GENERAL ENG RPM",              unit = "RPM",     kind = "A", index = 1)]
    eng1_rpm: f64,

    #[var(name = "L:MY_CUSTOM_VALUE",              unit = "Number",  kind = "L")]
    custom: f64,
}

let snapshot = FlightData::read()?;
println!("Alt: {} ft  Hdg: {}°  ENG1: {} RPM", snapshot.altitude_ft, snapshot.heading_deg, snapshot.eng1_rpm);
```

---

### Comm Bus — `msfs::comm_bus`

Send and receive binary messages between WASM modules, JavaScript, and the sim.

```rust
use msfs::prelude::*;

// Subscribe to a named event
let _sub = Subscription::subscribe("my.module/event", |bytes| {
    println!("Received {} bytes", bytes.len());
})?;

// Broadcast a message
let payload = 42u32.to_le_bytes();
commbus_call("my.module/event", &payload, BroadcastFlags::JS | BroadcastFlags::WASM);
```

**Broadcast flags:**

| Flag | Target |
|---|---|
| `BroadcastFlags::JS` | JavaScript gauges |
| `BroadcastFlags::WASM` | Other WASM modules |
| `BroadcastFlags::WASM_SELF` | This WASM module itself |
| `BroadcastFlags::ALL` | Everyone |
| `BroadcastFlags::DEFAULT` | SDK default |

Subscriptions automatically unsubscribe when dropped.

---

### HTTP Networking — `msfs::network`

Make asynchronous HTTP GET and POST requests from within a WASM module.

```rust
use msfs::prelude::*;

let params = HttpParams {
    headers: vec!["Accept: application/json".to_string()],
    post_field: None,
    body: vec![],
};

http_request(Method::Get, "https://example.com/data.json", params, |resp| {
    if resp.error_code == 0 {
        let body = String::from_utf8_lossy(&resp.data);
        println!("Got: {body}");
    }
})?;
```

Callbacks are invoked on the next simulator update tick after the response arrives.

---

### File I/O — `msfs::io`

#### High-level API (`msfs::io::fs`)

```rust
use msfs::io::fs;

// Async read — callback fires when data is ready
let req = fs::read("\\work/config.json", |data| {
    println!("Read {} bytes", data.len());
})?;

// Async write
let req = fs::write("\\work/output.bin", &my_bytes)?;

// Poll completion in your update loop
if req.is_done() { /* ... */ }
if req.has_error() { eprintln!("{:?}", req.last_error()); }
```

#### Low-level API (`msfs::io`)

Full control via `OpenFile`, `IoRequest`, and `OpenFlags` for advanced use cases.

**Supported open flags:** `RDONLY`, `WRONLY`, `RDWR`, `CREAT`, `TRUNC`, `HIDDEN`

---

### NanoVG Rendering — `msfs::nvg`

Vector graphics rendering inside a `Gauge` using the NanoVG API.

```rust
use msfs::nvg::*;
use msfs::prelude::*;

// In Gauge::init
let nvg = NvgContext::new(ctx).expect("NVG init failed");
let font = nvg.create_font("sans", "./data/Roboto-Regular.ttf");

// In Gauge::draw
nvg.frame(win_w, win_h, px_ratio, |nvg| {
    // Shapes
    Shape::rect(10.0, 10.0, 200.0, 100.0)
        .fill(Color::rgb(0, 120, 255))
        .draw(nvg);

    Shape::circle(150.0, 150.0, 40.0)
        .stroke(Color::rgba(255, 255, 255, 200), 2.0)
        .draw(nvg);

    // Text
    nvg.text(font_id, 16.0, 50.0, 50.0, "Hello MSFS!");

    // Transforms
    nvg.scoped(|nvg| {
        nvg.translate(cx, cy);
        nvg.rotate(angle_rad);
        // ... draw rotated content ...
    });
});
```

`NvgContext` is automatically cleaned up via `Drop`.

---

## Examples

All examples are in [`msfs/examples/`](msfs/examples/).

| Example | Description |
|---|---|
| [`io_system_simple.rs`](msfs/examples/io_system_simple.rs) | Read and copy a file using the high-level `io::fs` API |
| [`io_system.rs`](msfs/examples/io_system.rs) | Full low-level file I/O API |
| [`vars_full_api.rs`](msfs/examples/vars_full_api.rs) | A-vars, L-vars, indexed vars, `VarStruct` snapshot |
| [`comm_bus_gauge.rs`](msfs/examples/comm_bus_gauge.rs) | Gauge that publishes Comm Bus events on mouse click |
| [`comm_bus_sytem.rs`](msfs/examples/comm_bus_sytem.rs) | System that receives Comm Bus commands and broadcasts state |
| [`network_fetch_system.rs`](msfs/examples/network_fetch_system.rs) | Fetch JSON over HTTP on a Comm Bus trigger |
| [`network_post_system.rs`](msfs/examples/network_post_system.rs) | HTTP POST with a request body |
| [`nvg_render.rs`](msfs/examples/nvg_render.rs) | Attitude indicator rendered with NanoVG |

---

## Work in Progress

The following SDK headers are planned for binding but are not yet implemented. Tracking them here so users know what to expect.

| SDK Header | Module (planned) | Description |
|---|---|---|
| `MSFS_Events.h` | `msfs::events` | Subscribe to and fire named simulation events |
| `MSFS_MapView.h` | `msfs::map_view` | Render interactive map views in NVG (weather/terrain radars) |
| `MSFS_Vfx.h` | `msfs::vfx` | Spawn VFX taht are defined |
| `MSFS_Weather.h` | `msfs::weather` | Read and manipulate weather conditions and METAR data |
| `MSFS_PlannedRoute.h` | `msfs::planned_route` | Work with the native flight plans |
| `MSFS_Charts.h` | `msfs::charts` | Fetch built in FAA and LIDO charts from sim |

If you need one of these APIs before official bindings land, the raw FFI symbols are already available through `msfs::sys` (generated by bindgen from the SDK headers at build time).

---

## Crate Structure

```
msfs/src/
├── lib.rs          — top-level re-exports
├── prelude.rs      — convenient glob import
├── modules.rs      — System / Gauge traits
├── exports.rs      — export_system! / export_gauge! macros
├── context.rs      — FsContext wrapper
├── types.rs        — GaugeDraw, GaugeInstall, SystemInstall
├── sys.rs          — raw bindgen bindings
├── vars/           — AVar, LVar, VarKind, VarStruct
├── comm_bus/       — Subscription, BroadcastFlags, commbus_call
├── network/        — http_request, HttpParams, Method, HttpResponse
├── io/             — File I/O (low-level + fs high-level)
├── nvg/            — NanoVG: NvgContext, Shape, Color, Transform, …
├── events/         — Sim event helpers
├── utils/          — Internal utilities
└── bindgen_support/— Headers consumed by the build script
```



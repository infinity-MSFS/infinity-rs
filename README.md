# infinity-rs

Safe, idiomatic Rust bindings for the **Microsoft Flight Simulator 2024 (MSFS 2024) WASM SDK**.

Write MSFS gauges and systems in Rust with safe, idiomatic wrappers over the entire WASM SDK surface — SimVars, Comm Bus, async HTTP, file I/O, NanoVG rendering, simulation events, the flow lifecycle, map views (with weather radar / terrain), airport charts, VFX, and the EFB planned route channel.

---

## Workspace Layout

| Crate | Package name | Description |
|---|---|---|
| `msfs/` | `infinity-rs` | Main bindings crate — re-exports everything you need |
| `msfs_derive/` | `msfs_derive` | Proc-macro helpers (`#[derive(VarStruct)]`) |
| `msfs_sdk/` | `msfs_sdk` | Build helper that locates the installed MSFS 2024 SDK |
| `build-tools/` | `infinity-msfs` | CLI for building / packaging WASM modules |

> The lib crate is named **`infinity-rs`** (Rust path: `infinity_rs`). It used to be `msfs`, which collided with the FlyByWire crate of the same name; the rename in `0.2.0` is intentionally breaking. Update imports from `use msfs::...` to `use infinity_rs::...`, and `msfs::export_gauge!` / `msfs::export_system!` to `infinity_rs::export_gauge!` / `infinity_rs::export_system!`.

---

## Prerequisites

### WASM Builds (normal usage)

The `infinity-msfs` build tools work on **any platform** (Windows, macOS, Linux) and do **not** require the MSFS 2024 SDK. The necessary MSFS headers and WASI sysroot are bundled and installed automatically.

| Requirement | Notes |
|---|---|
| Rust nightly / `wasm32-wasip1` target | `rustup target add wasm32-wasip1` |
| `clang` + `llvm-ar` | Required for WASM compilation; install via [LLVM](https://releases.llvm.org/) or your system package manager |
| `wasm-opt` | Part of [Binaryen](https://github.com/WebAssembly/binaryen); only required when `wasm_opt.enabled = true` in your config |

### SimConnect / Native Builds

The MSFS 2024 SDK is **only** required when building for a native target with SimConnect support (e.g. external tooling that talks to a running sim instance):

| Requirement | Notes |
|---|---|
| **MSFS 2024 SDK** | Install via the MSFS Dev Mode or the standalone SDK installer |
| `MSFS2024_SDK` env var | Must point to your SDK root (e.g. `C:\MSFS 2024 SDK`) |

The build script automatically detects whether you are targeting `wasm32` and adjusts compiler flags, NanoVG compilation, and linking accordingly.

---

## Getting Started

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
infinity-rs = { path = "path/to/infinity-rs/msfs" }
msfs_derive = { path = "path/to/infinity-rs/msfs_derive" }  # only needed for VarStruct
```

In code:

```rust
use infinity_rs::prelude::*;
```

Set the SDK environment variable before building:

```powershell
$env:MSFS2024_SDK = "C:\MSFS 2024 SDK"
cargo build --target wasm32-wasip1
```

---

## Core Concepts


### Build Tools
The build tools supplied with the project compile Rust WASM targets, optionally run `wasm-opt`, and can list the configured projects before building. Copy [`infinity-msfs.toml.example`](infinity-msfs.toml.example) to `infinity-msfs.toml` in your project root and adjust the package names and copy rules for your own workspace.

```toml
[build]
target = "wasm32-wasip1"
out_dir = "build/msfs"

copy = [
  { from = "manifest.json", to = "build/msfs/manifest.json" },
  { from = "layout.json", to = "build/msfs/layout.json" }
]

[[packages]]
package = "demo-gauge"
bin = "demo_gauge"
out_name = "demo-gauge.wasm"

[[packages]]
package = "demo-system"
bin = "demo_system"
out_name = "demo-system.wasm"
out_dir = "build/msfs/systems"

[wasm_opt]
enabled = true
args = [
  "-O1",
  "--signext-lowering",
  "--enable-bulk-memory",
  "--enable-nontrapping-float-to-int",
]

[scripts]
pre_build = [
  "cargo fmt --check"
]

post_build = [
  "python ./tools/fix_layout.py"
]
```

some example build commands:
```bash
infinity-msfs projects
infinity-msfs list-projects
infinity-msfs build
infinity-msfs build --release
infinity-msfs build --release --no-wasm-opt
infinity-msfs build --only demo-gauge --release
infinity-msfs build --verbose
```

### Systems and Gauges

Everything in MSFS runs as either a **System** or a **Gauge**. Implement the corresponding trait and export it with the matching macro.

**System** — logic-only, no rendering:

```rust
use infinity_rs::prelude::*;

pub struct MySystem { /* ... */ }

impl System for MySystem {
    fn init(&mut self, ctx: &Context, install: &SystemInstall) -> bool { true }
    fn update(&mut self, ctx: &Context, dt: f32) -> bool { true }
    fn kill(&mut self, ctx: &Context) -> bool { true }
}

infinity_rs::export_system!(
    name  = my_system,
    state = MySystem,
    ctor  = MySystem::new(),
);
```

**Gauge** — rendered panel element with optional mouse input:

```rust
use infinity_rs::prelude::*;

pub struct MyGauge { /* ... */ }

impl Gauge for MyGauge {
    fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool { true }
    fn update(&mut self, ctx: &Context, dt: f32) -> bool { true }
    fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool { true }
    fn kill(&mut self, ctx: &Context) -> bool { true }

    fn mouse(&mut self, _ctx: &Context, x: f32, y: f32, flags: i32) { /* optional */ }
}

infinity_rs::export_gauge!(
    name  = my_gauge,
    state = MyGauge,
    ctor  = MyGauge::new(),
);
```

The macros emit the correctly named `extern "C"` entry points expected by the simulator.

---

## Features

### SimVars — `infinity_rs::vars`

Read and write simulation variables via `AVar` (A-vars) and `LVar` (L-vars).

```rust
use infinity_rs::vars::{AVar, LVar};

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
use infinity_rs::vars::{AVar, VarParamArray1};

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

### Comm Bus — `infinity_rs::comm_bus`

Send and receive binary messages between WASM modules, JavaScript, and the sim.

```rust
use infinity_rs::prelude::*;

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

### HTTP Networking — `infinity_rs::network`

Make asynchronous HTTP GET and POST requests from within a WASM module.

```rust
use infinity_rs::prelude::*;

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

### File I/O — `infinity_rs::io`

#### High-level API (`infinity_rs::io::fs`)

```rust
use infinity_rs::io::fs;

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

#### Low-level API (`infinity_rs::io`)

Full control via `OpenFile`, `IoRequest`, and `OpenFlags` for advanced use cases.

**Supported open flags:** `RDONLY`, `WRONLY`, `RDWR`, `CREAT`, `TRUNC`, `HIDDEN`

---

### NanoVG Rendering — `infinity_rs::nvg`

Vector graphics rendering inside a `Gauge` using the NanoVG API.

```rust
use infinity_rs::nvg::*;
use infinity_rs::prelude::*;

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

### Flow API — `infinity_rs::flow`
Subscribe to simulation lifecycle events (flight load, teleport start/done, replay boundaries, plane crash, …). Useful for resetting state on flight reload or pausing logic across slew/back-on-track windows. Subscriptions auto-unregister on `Drop`.

### Event API — `infinity_rs::events`
Subscribe to and trigger named simulation events (`KEY_TOGGLE_MASTER_BATTERY`, etc.) with typed parameter arguments via `FsParamArg`.

### Map Views — `infinity_rs::map_view`
`MapView` wraps `fsMapView*`: spawn a host-side map texture, configure aerial / altitude shading, follow mode, isolines, and an integrated weather radar (top / horizontal / vertical with custom rain-rate color ramps and cone angle). `MapView::image_pattern(...)` hands the texture straight to NanoVG so you can paint it through any gauge.

### Charts — `infinity_rs::charts`
Async access to the `fsCharts*` API. `get_index → get_pages → get_page_image` returns a `ChartImage` whose host id can be sampled by NanoVG via `nvg_pattern(...)`. `ChartIndex` and `ChartPages` are owned wrappers that release the host allocations on `Drop`; ref-views (`ChartCategoryRef`, `ChartMetadataRef`, `ChartPageRef`) decode strings as `&str` borrowed from the owner.

### VFX — `infinity_rs::vfx`
Spawn particle effects in the world (`spawn_in_world`) or attached to a sim object node (`spawn_on_sim_object`) with optional `VfxParam { name, rpn }` graph bindings. `VfxInstance` owns the host id and destroys it on `Drop`; `leak()` detaches if the host should keep emitting after the handle goes away.

### Planned Route — `infinity_rs::planned_route`
EFB / route-broadcast integration. `current_efb_route()` borrows the route the EFB currently holds; `subscribe_broadcast(...)` listens for pushes; `subscribe_requests(...)` receives `RouteRequest` handles when other modules ask for a route, and you reply via `request.respond(route)`. Both subscriptions auto-unregister on `Drop`.

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
| [`flow_system.rs`](msfs/examples/flow_system.rs) | Adjust logic based on sim events using the Flow API |
| [`event_api.rs`](msfs/examples/event_api.rs) | Subscribe to and trigger simulation events |
| [`map_view_weather_radar.rs`](msfs/examples/map_view_weather_radar.rs) | Texture-backed weather radar gauge with rain-rate color ramp + range overlay |
| [`charts_gauge.rs`](msfs/examples/charts_gauge.rs) | Async charts pipeline (index → pages → image) painted through NanoVG |
| [`vfx_system.rs`](msfs/examples/vfx_system.rs) | Sim-object-attached particle effect driven by N1 via an RPN-bound graph param |
| [`planned_route_system.rs`](msfs/examples/planned_route_system.rs) | Subscribe to EFB route broadcasts and respond to route requests |

---


## Crate Structure

```
msfs/src/
├── lib.rs            — top-level re-exports
├── prelude.rs        — convenient glob import
├── modules.rs        — System / Gauge traits
├── exports.rs        — export_system! / export_gauge! macros
├── context.rs        — FsContext wrapper
├── types.rs          — GaugeDraw, GaugeInstall, SystemInstall
├── sys.rs            — raw bindgen bindings
├── abi.rs            — ABI shims for host entry points
├── vars/             — AVar, LVar, VarKind, VarStruct
├── comm_bus/         — Subscription, BroadcastFlags, commbus_call
├── network/          — http_request, HttpParams, Method, HttpResponse
├── io/               — File I/O (low-level + fs high-level)
├── nvg/              — NanoVG: NvgContext, Shape, Color, Transform, …
├── events/           — Sim event helpers (subscribe / trigger / FsParamArg)
├── flow/             — Flow lifecycle subscription
├── map_view.rs       — MapView (fsMapView*) — weather radar / terrain / 3D
├── charts.rs         — Async charts API (fsCharts*) with owned wrappers
├── vfx.rs            — VfxInstance (fsVfx*) with RAII destroy
├── planned_route.rs  — EFB planned-route broadcast + request channels
├── host/             — Native testing shims (non-wasm targets)
├── utils/            — Internal utilities
└── bindgen_support/  — Headers consumed by the build script
```

---

## SDK Coverage

Every public header in `MSFS 2024 SDK/WASM/include/MSFS/` is wrapped:

| Header | Module |
|---|---|
| `MSFS.h`, `MSFS_Core.h`, `MSFS_Utils.h` | `infinity_rs::sys`, internal helpers |
| `MSFS_Vars.h` | `infinity_rs::vars` |
| `MSFS_CommBus.h` | `infinity_rs::comm_bus` |
| `MSFS_Network.h` | `infinity_rs::network` |
| `MSFS_IO.h` | `infinity_rs::io` |
| `MSFS_Render.h`, `Render/nanovg.h` | `infinity_rs::nvg` |
| `MSFS_GaugeContext.h`, `MSFS_SystemContext.h` | `infinity_rs::context`, `infinity_rs::modules` |
| `MSFS_Events.h`, `Types/MSFS_EventsEnum.h` | `infinity_rs::events` |
| `MSFS_Flow.h` | `infinity_rs::flow` |
| `MSFS_MapView.h` | `infinity_rs::map_view` |
| `MSFS_Charts.h` | `infinity_rs::charts` |
| `MSFS_Vfx.h` | `infinity_rs::vfx` |
| `MSFS_PlannedRoute.h`, `MSFS_FlightPlan.h` | `infinity_rs::planned_route` (FlightPlan types re-exposed via `sys`) |
| `MSFS_Weather.h` | re-exported via `infinity_rs::sys` (typed wrapper TBD) |
| `Legacy/gauges.h` | `infinity_rs::sys` (legacy gauge ABI) |
| `SimConnect.h` | `infinity_rs::sys` under the `simconnect` feature (native only) |

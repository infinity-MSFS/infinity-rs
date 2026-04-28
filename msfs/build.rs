fn main() {
    let wasm = std::env::var("TARGET").unwrap().starts_with("wasm32-");
    let simconnect = std::env::var("CARGO_FEATURE_SIMCONNECT").is_ok();
    let msfs_sdk = msfs_sdk::msfs_sdk_path().unwrap();

    if wasm {
        unsafe {
            std::env::set_var("AR", "llvm-ar");
        }
        cc::Build::new()
            .compiler("clang")
            .flag(format!("--sysroot={msfs_sdk}/WASM/wasi-sysroot"))
            .flag("-fms-extensions") // intended to be used with msvc
            .flag("-D__INTELLISENSE__") // get rid of incorrect __attribute__'s from asobo
            .flag("-Wno-unused-parameter") // warning in nanovg
            .flag("-Wno-sign-compare") // warning in nanovg
            .flag("-mthread-model") // no thread support
            .flag("single") // no thread support
            .include(format!("{msfs_sdk}/WASM/include"))
            .file(format!("{msfs_sdk}/WASM/src/MSFS/Render/nanovg.cpp"))
            .compile("nanovg");

        // Link MSFS_WasmVersions.a so the resulting module is detected as
        // an MSFS 2024 module (required for SimConnect and other 2024-era
        // features). Without this the runtime falls back to MSFS 2020
        // behavior.
        // The SDK ships the file as `MSFS_WasmVersions.a` (no `lib` prefix),
        // so the `+verbatim` modifier is required — without it rustc looks
        // for `libMSFS_WasmVersions.a` and fails.
        println!("cargo:rustc-link-search=native={msfs_sdk}/WASM/WasmVersions");
        println!("cargo:rustc-link-lib=static:+verbatim=MSFS_WasmVersions.a");
    }

    {
        println!("cargo:rerun-if-changed=bindgen/wrapper.h");
        println!("cargo:rerun-if-changed=bindgen/shims");
        let mut bindings = bindgen::Builder::default()
            .clang_arg(format!("-I{msfs_sdk}/WASM/include"))
            .clang_arg("-Ibindgen/shims")
            .clang_arg("-fms-extensions")
            .clang_arg("-fvisibility=default")
            .clang_arg("-xc++")
            .clang_arg("-std=c++17")
            .clang_arg("-v")
            .header("bindgen/wrapper.h")
            // Required under edition 2024: bindgen-generated `unsafe fn`s
            // call other unsafe fns, which is now a hard error unless the
            // body wraps them in an explicit `unsafe { … }` block.
            .wrap_unsafe_ops(true)
            // .blocklist_function("nvgFillColor")
            // .blocklist_function("nvgFillPaint")
            // .blocklist_function("nvgStrokeColor")
            // .blocklist_function("nvgStrokePaint")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .impl_debug(false)
            // `opaque_type` added to avoid alignment errors. These alignment errors are caused
            // because virtual methods are not well supported in rust-bindgen.
            .opaque_type("IGaugeCDrawableCreateParameters")
            .opaque_type("IGaugeCDrawableDrawParameters")
            .opaque_type("IGaugeCDrawable")
            .opaque_type("IGaugeCCallback")
            .opaque_type("ISerializableGaugeCCallback")
            .opaque_type("IAircraftCCallback")
            .opaque_type("IPanelCCallback")
            .opaque_type("IFSXPanelCCallback");

        if simconnect {
            bindings = bindings
                .clang_arg(format!("-I{msfs_sdk}/SimConnect SDK/include"))
                .clang_arg("-DUSE_SIMCONNECT=1")
                .rustified_enum("SIMCONNECT_EXCEPTION")
                .rustified_enum("SIMCONNECT_DATATYPE")
                .rustified_enum("SIMCONNECT_PERIOD")
                .rustified_enum("SIMCONNECT_CLIENT_DATA_PERIOD")
                .rustified_enum("SIMCONNECT_SIMOBJECT_TYPE")
                .rustified_enum("SIMCONNECT_STATE")
                .rustified_enum("SIMCONNECT_TEXT_TYPE")
                .rustified_enum("SIMCONNECT_TEXT_RESULT")
                .rustified_enum("SIMCONNECT_WEATHER_MODE")
                .rustified_enum("SIMCONNECT_FACILITY_LIST_TYPE")
                .rustified_enum("SIMCONNECT_FACILITY_DATA_TYPE")
                .rustified_enum("SIMCONNECT_INPUT_EVENT_TYPE")
                .rustified_enum("SIMCONNECT_FLOW_EVENT")
                .rustified_enum("SIMCONNECT_MISSION_END");
        }

        if wasm {
            bindings = bindings.clang_arg("-D_MSFS_WASM 1");
        }

        bindings
            .generate()
            .unwrap()
            .write_to_file(
                std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("msfs-sys.rs"),
            )
            .unwrap();
    }

    if !wasm && simconnect {
        println!("cargo:rustc-link-search={msfs_sdk}/SimConnect SDK/lib/static");
        println!("cargo:rustc-link-lib=SimConnect");
        println!("cargo:rustc-link-lib=shlwapi");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=advapi32");
    }
}

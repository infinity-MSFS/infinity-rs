fn main() {
    let wasm = std::env::var("TARGET").unwrap().starts_with("wasm32-");
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
    }

    {
        println!("cargo:rerun-if-changed=src/bindgen_support/wrapper.h");
        let mut bindings = bindgen::Builder::default()
            .clang_arg(format!("-I{msfs_sdk}/WASM/include"))
            .clang_arg(format!("-I{msfs_sdk}/SimConnect SDK/include"))
            .clang_arg(format!("-I{}", "src/bindgen_support"))
            .clang_arg("-fms-extensions")
            .clang_arg("-fvisibility=default")
            .clang_arg("-xc++")
            .clang_arg("-std=c++17")
            .clang_arg("-v")
            .header("src/bindgen_support/wrapper.h")
            // .blocklist_function("nvgFillColor")
            // .blocklist_function("nvgFillPaint")
            // .blocklist_function("nvgStrokeColor")
            // .blocklist_function("nvgStrokePaint")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .rustified_enum("SIMCONNECT_EXCEPTION")
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

    if !wasm {
        println!("cargo:rustc-link-search={msfs_sdk}/SimConnect SDK/lib/static");
        println!("cargo:rustc-link-lib=SimConnect");
        println!("cargo:rustc-link-lib=shlwapi");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=shell32");

        // If the nanovg-shim feature is enabled, build the bundled CMake shim and copy the DLL
        // next to produced binaries (target/{profile}).
        if std::env::var_os("CARGO_FEATURE_NANOVG_SHIM").is_some() {
            let target = std::env::var("TARGET").unwrap();
            let is_windows = target.contains("windows");

            if is_windows {
                println!("cargo:rerun-if-changed=../nvg_shim/CMakeLists.txt");
                println!("cargo:rerun-if-changed=../nvg_shim/shim_impl.cpp");
                println!("cargo:rerun-if-changed=../nvg_shim/nvg/nanovg.h");
                println!("cargo:rerun-if-changed=../nvg_shim/nvg/nanovg.cpp");
                println!("cargo:rerun-if-changed=../nvg_shim/nvg/nanovg_sw.h");

                let manifest_dir =
                    std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
                let project_root = manifest_dir
                    .parent()
                    .expect("msfs crate must live under workspace root");

                let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

                // Run the helper PowerShell script.
                let script = manifest_dir.join("build_nvg_shim.ps1");
                let status = std::process::Command::new("pwsh")
                    .arg("-NoProfile")
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-File")
                    .arg(&script)
                    .arg("-ProjectRoot")
                    .arg(project_root)
                    .arg("-OutDir")
                    .arg(&out_dir)
                    .status()
                    .expect("failed to execute build_nvg_shim.ps1");

                if !status.success() {
                    panic!("failed to build nvg_shim (cmake)");
                }

                // Copy the produced DLL into target/{profile} so examples can load it without extra setup.
                // This mirrors typical MSVC runtime loading behavior.
                let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
                let target_dir = project_root.join("target").join(&profile);
                std::fs::create_dir_all(&target_dir).ok();

                let built_dll = out_dir.join("nanovg_shim.dll");
                if built_dll.exists() {
                    let dest = target_dir.join("nanovg_shim.dll");
                    let _ = std::fs::copy(&built_dll, &dest);
                }
            }
        }
    }
}

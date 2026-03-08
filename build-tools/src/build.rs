use crate::{
    cargo_meta,
    cli::BuildArgs,
    config::{CopyRule, InfinityMsfsToml},
    scripts, util,
};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn run_build(args: BuildArgs) -> Result<()> {
    let root = util::find_project_root()?;
    let config_path = util::config_path(&root);

    let cfg = if config_path.exists() {
        InfinityMsfsToml::load(&config_path)?
    } else {
        InfinityMsfsToml::default()
    };

    let metadata = cargo_meta::load_metadata(&root)?;

    let package_name = args.package.clone().or_else(|| cfg.build.package.clone());

    let package = cargo_meta::resolve_package(&metadata, package_name.as_deref())?;
    let bin_name = cargo_meta::resolve_bin_name(package, cfg.build.bin.as_deref());

    let target = cfg.build.target.clone();
    let out_dir = root.join(&cfg.build.out_dir);
    let out_name = cfg
        .build
        .out_name
        .clone()
        .unwrap_or_else(|| format!("{bin_name}.wasm"));

    let built_wasm = built_wasm_path(&root, &target, args.release, &bin_name);
    let final_wasm = out_dir.join(out_name);

    let use_wasm_opt = cfg.wasm_opt.enabled && !args.no_wasm_opt;

    println!("[infinity-msfs] root: {}", root.display());
    println!("[infinity-msfs] package: {}", package.name);
    println!("[infinity-msfs] bin: {}", bin_name);
    println!("[infinity-msfs] target: {}", target);
    println!(
        "[infinity-msfs] profile: {}",
        if args.release { "release" } else { "debug" }
    );
    println!(
        "[infinity-msfs] wasm-opt: {}",
        if use_wasm_opt { "enabled" } else { "disabled" }
    );
    println!("[infinity-msfs] output: {}", final_wasm.display());

    scripts::run_script_list(&root, "pre_build", &cfg.scripts.pre_build)?;

    run_cargo_build(&root, &target, &package.name, args.release)?;

    if !built_wasm.exists() {
        bail!(
            "cargo build completed, but built wasm was not found at {}",
            built_wasm.display()
        );
    }

    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create output directory {}", out_dir.display()))?;

    if use_wasm_opt {
        run_wasm_opt(&root, &cfg.wasm_opt.args, &built_wasm, &final_wasm)?;
    } else {
        util::copy_file(&built_wasm, &final_wasm)?;
    }

    run_copy_rules(&root, &cfg.build.copy)?;

    scripts::run_script_list(&root, "post_build", &cfg.scripts.post_build)?;

    println!("[infinity-msfs] done");
    Ok(())
}

fn built_wasm_path(root: &Path, target: &str, release: bool, bin_name: &str) -> PathBuf {
    let profile = if release { "release" } else { "debug" };

    let bin_name = bin_name.replace('-', "_");

    root.join("target")
        .join(target)
        .join(profile)
        .join(format!("{bin_name}.wasm"))
}

fn run_cargo_build(root: &Path, target: &str, package: &str, release: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root)
        .arg("build")
        .arg("--target")
        .arg(target)
        .arg("-p")
        .arg(package);

    if release {
        cmd.arg("--release");
    }

    run_command(&mut cmd, "cargo build")
}

fn run_wasm_opt(root: &Path, opt_args: &[String], input: &Path, output: &Path) -> Result<()> {
    let mut cmd = Command::new("wasm-opt");
    cmd.current_dir(root);

    for arg in opt_args {
        cmd.arg(arg);
    }

    cmd.arg("-o").arg(output).arg(input);

    run_command(&mut cmd, "wasm-opt")
}

fn run_copy_rules(root: &Path, rules: &[CopyRule]) -> Result<()> {
    for rule in rules {
        let from = root.join(&rule.from);
        let to = root.join(&rule.to);

        if !from.exists() {
            bail!(
                "copy source does not exist: {} (configured destination: {})",
                from.display(),
                to.display()
            );
        }

        println!(
            "[infinity-msfs] copying {} -> {}",
            from.display(),
            to.display()
        );

        util::copy_file(&from, &to)?;
    }
    Ok(())
}

fn run_command(cmd: &mut Command, label: &str) -> Result<()> {
    println!("[infinity-msfs] running: {cmd:?}");

    let status = cmd
        .status()
        .with_context(|| format!("failed to start {label}"))?;

    if !status.success() {
        bail!("{label} failed with status {status}");
    }
    Ok(())
}

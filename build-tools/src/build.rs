use crate::{
    cargo_meta,
    cli::{BuildArgs, ProjectsArgs},
    config::{BuildConfig, CopyRule, InfinityMsfsToml, PackageBuild},
    process, scripts, setup,
    ui::{self, BuildOutcome, BuildUi},
    util,
};
use anyhow::{Context, Result, bail};
use cargo_metadata::Metadata;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Resolved, ready-to-execute build description for a single wasm artefact.
struct BuildPlan {
    package: String,
    bin: String,
    target: String,
    out_dir: PathBuf,
    out_name: String,
    copy: Vec<CopyRule>,
}

pub fn run_build(args: BuildArgs) -> Result<()> {
    setup::ensure_sdk_headers()?;

    let root = util::find_project_root()?;
    let config_path = util::config_path(&root);

    let cfg = if config_path.exists() {
        InfinityMsfsToml::load(&config_path)?
    } else {
        InfinityMsfsToml::default()
    };

    let metadata = cargo_meta::load_metadata(&root)?;

    let plans = resolve_plans(&root, &cfg, &metadata, args.package.as_deref(), &args.only)?;

    if plans.is_empty() {
        bail!("no packages selected to build");
    }

    let use_wasm_opt = cfg.wasm_opt.enabled && !args.no_wasm_opt;
    let mut ui = BuildUi::new(&root, plans.len(), args.release, use_wasm_opt, args.verbose);

    ui.announce_phase("Running pre-build scripts", cfg.scripts.pre_build.len());
    scripts::run_script_list(&root, "pre_build", &cfg.scripts.pre_build, args.verbose)?;

    for plan in &plans {
        ui.start_package(&plan.package);
        let outcome = build_one(
            &root,
            plan,
            &cfg.wasm_opt.args,
            use_wasm_opt,
            args.release,
            args.verbose,
        )?;
        ui.finish_package(&plan.package, &plan.out_dir.join(&plan.out_name), outcome);
    }

    ui.announce_phase("Running post-build scripts", cfg.scripts.post_build.len());
    scripts::run_script_list(&root, "post_build", &cfg.scripts.post_build, args.verbose)?;

    ui.finish();
    Ok(())
}

pub fn run_projects(args: ProjectsArgs) -> Result<()> {
    let root = util::find_project_root()?;
    let config_path = util::config_path(&root);

    let cfg = if config_path.exists() {
        InfinityMsfsToml::load(&config_path)?
    } else {
        InfinityMsfsToml::default()
    };

    let metadata = cargo_meta::load_metadata(&root)?;
    let plans = resolve_plans(&root, &cfg, &metadata, args.package.as_deref(), &args.only)?;

    if plans.is_empty() {
        bail!("no packages selected to list");
    }

    ui::print_projects(
        root.as_path(),
        plans.into_iter().map(|plan| {
            (
                plan.package,
                plan.bin,
                plan.target,
                plan.out_dir.join(plan.out_name),
            )
        }),
    );

    Ok(())
}

fn build_one(
    root: &Path,
    plan: &BuildPlan,
    wasm_opt_args: &[String],
    use_wasm_opt: bool,
    release: bool,
    verbose: bool,
) -> Result<BuildOutcome> {
    let built_wasm = built_wasm_path(root, &plan.target, release, &plan.bin);
    let final_wasm = plan.out_dir.join(&plan.out_name);

    run_cargo_build(root, &plan.target, &plan.package, release, verbose)?;

    if !built_wasm.exists() {
        bail!(
            "cargo build completed, but built wasm was not found at {}",
            built_wasm.display()
        );
    }

    fs::create_dir_all(&plan.out_dir).with_context(|| {
        format!(
            "failed to create output directory {}",
            plan.out_dir.display()
        )
    })?;

    if use_wasm_opt {
        run_wasm_opt(root, wasm_opt_args, &built_wasm, &final_wasm, verbose)?;
    } else {
        util::copy_file(&built_wasm, &final_wasm)?;
    }

    let copied_files = run_copy_rules(root, &plan.copy)?;

    Ok(BuildOutcome { copied_files })
}

/// Build a list of `BuildPlan`s honouring `[[packages]]` when present and
/// falling back to the legacy single-`[build]` path otherwise. Applies any
/// CLI filters (`-p` / `--only`) before returning.
fn resolve_plans(
    root: &Path,
    cfg: &InfinityMsfsToml,
    metadata: &Metadata,
    cli_package: Option<&str>,
    only: &[String],
) -> Result<Vec<BuildPlan>> {
    let mut plans: Vec<BuildPlan> = if !cfg.packages.is_empty() {
        cfg.packages
            .iter()
            .map(|entry| plan_from_package_entry(root, metadata, &cfg.build, entry))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![plan_from_legacy(root, metadata, &cfg.build, cli_package)?]
    };

    let filters = collect_filters(cli_package, only);
    if !filters.is_empty() {
        let before = plans.len();
        plans.retain(|p| filters.iter().any(|f| f == &p.package));
        if plans.is_empty() {
            bail!(
                "no configured package matched filter {:?} (had {} candidate{})",
                filters,
                before,
                if before == 1 { "" } else { "s" }
            );
        }
    }

    Ok(plans)
}

fn collect_filters(cli_package: Option<&str>, only: &[String]) -> Vec<String> {
    let mut out: Vec<String> = only.to_vec();
    if let Some(p) = cli_package {
        if !out.iter().any(|candidate| candidate == p) {
            out.push(p.to_string());
        }
    }
    out
}

fn plan_from_package_entry(
    root: &Path,
    metadata: &Metadata,
    base: &BuildConfig,
    entry: &PackageBuild,
) -> Result<BuildPlan> {
    let pkg = cargo_meta::resolve_package(metadata, Some(&entry.package))?;
    let bin = cargo_meta::resolve_bin_name(pkg, entry.bin.as_deref().or(base.bin.as_deref()));

    let target = entry.target.clone().unwrap_or_else(|| base.target.clone());
    let out_dir_rel = entry
        .out_dir
        .clone()
        .unwrap_or_else(|| base.out_dir.clone());
    let out_dir = root.join(&out_dir_rel);

    let out_name = entry
        .out_name
        .clone()
        .or_else(|| {
            // Inherit top-level out_name only when the top-level build.package
            // matches this entry — otherwise we would overwrite the same file
            // for every package in the list.
            match &base.package {
                Some(bp) if bp == &entry.package => base.out_name.clone(),
                _ => None,
            }
        })
        .unwrap_or_else(|| format!("{bin}.wasm"));

    let mut copy = base.copy.clone();
    copy.extend(entry.copy.iter().cloned());

    Ok(BuildPlan {
        package: pkg.name.clone(),
        bin,
        target,
        out_dir,
        out_name,
        copy,
    })
}

fn plan_from_legacy(
    root: &Path,
    metadata: &Metadata,
    base: &BuildConfig,
    cli_package: Option<&str>,
) -> Result<BuildPlan> {
    let package_name = cli_package
        .map(|s| s.to_string())
        .or_else(|| base.package.clone());

    let pkg = cargo_meta::resolve_package(metadata, package_name.as_deref())?;
    let bin = cargo_meta::resolve_bin_name(pkg, base.bin.as_deref());

    let target = base.target.clone();
    let out_dir = root.join(&base.out_dir);
    let out_name = base
        .out_name
        .clone()
        .unwrap_or_else(|| format!("{bin}.wasm"));

    Ok(BuildPlan {
        package: pkg.name.clone(),
        bin,
        target,
        out_dir,
        out_name,
        copy: base.copy.clone(),
    })
}

fn built_wasm_path(root: &Path, target: &str, release: bool, bin_name: &str) -> PathBuf {
    let profile = if release { "release" } else { "debug" };

    let bin_name = bin_name.replace('-', "_");

    root.join("target")
        .join(target)
        .join(profile)
        .join(format!("{bin_name}.wasm"))
}

fn run_cargo_build(
    root: &Path,
    target: &str,
    package: &str,
    release: bool,
    verbose: bool,
) -> Result<()> {
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

    process::run_command(&mut cmd, "cargo build", verbose)?;
    Ok(())
}

fn run_wasm_opt(
    root: &Path,
    opt_args: &[String],
    input: &Path,
    output: &Path,
    verbose: bool,
) -> Result<()> {
    let mut cmd = Command::new("wasm-opt");
    cmd.current_dir(root);

    for arg in opt_args {
        cmd.arg(arg);
    }

    cmd.arg("-o").arg(output).arg(input);

    process::run_command(&mut cmd, "wasm-opt", verbose)?;
    Ok(())
}

fn run_copy_rules(root: &Path, rules: &[CopyRule]) -> Result<usize> {
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

        util::copy_file(&from, &to)?;
    }
    Ok(rules.len())
}

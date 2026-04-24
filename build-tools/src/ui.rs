use anyhow::Error;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct BuildUi {
    progress: ProgressBar,
    started: Instant,
    total: usize,
    copied_files: usize,
    verbose: bool,
}

pub struct BuildOutcome {
    pub copied_files: usize,
}

impl BuildUi {
    pub fn new(root: &Path, total: usize, release: bool, wasm_opt: bool, verbose: bool) -> Self {
        println!(
            "{} {} {} {} {}",
            style("Building").cyan().bold(),
            style(total).bold(),
            pluralize(total, "package"),
            style(format!(
                "({}; wasm-opt {})",
                if release { "release" } else { "debug" },
                if wasm_opt { "on" } else { "off" }
            ))
            .dim(),
            style(root.display()).dim()
        );

        let progress = ProgressBar::new(total as u64);
        progress.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}",
            )
            .expect("valid build progress template")
            .progress_chars("=> "),
        );

        if verbose {
            progress.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        } else {
            progress.enable_steady_tick(Duration::from_millis(120));
        }

        Self {
            progress,
            started: Instant::now(),
            total,
            copied_files: 0,
            verbose,
        }
    }

    pub fn announce_phase(&self, label: &str, count: usize) {
        if count == 0 {
            return;
        }

        let mut line = format!(
            "{} {} {}",
            style("→").cyan().bold(),
            style(label).bold(),
            style(format!("({count} {})", pluralize(count, "script"))).dim(),
        );
        if self.verbose {
            line.push(' ');
            line.push_str(&style("verbose").yellow().dim().to_string());
        }
        self.println(line);
    }

    pub fn start_package(&self, package: &str) {
        self.progress.set_message(format!("building {package}"));
    }

    pub fn finish_package(&mut self, package: &str, output: &Path, outcome: BuildOutcome) {
        self.copied_files += outcome.copied_files;

        self.progress.inc(1);
        self.println(format!(
            "{} {} {}{}",
            style("✓").green().bold(),
            style(package).bold(),
            style(shorten_path(output)).dim(),
            format_suffix(outcome.copied_files)
        ));
    }

    pub fn finish(self) {
        self.progress.finish_and_clear();

        let mut summary = format!(
            "{} built {} {} in {}",
            style("Done").green().bold(),
            style(self.total).bold(),
            pluralize(self.total, "package"),
            style(format_duration(self.started.elapsed())).dim()
        );

        if self.copied_files > 0 {
            summary.push_str(&format!(
                ", {} {}",
                style(self.copied_files).cyan().bold(),
                pluralize(self.copied_files, "copied file")
            ));
        }

        println!("{summary}");
    }

    fn println(&self, line: String) {
        if self.verbose {
            println!("{line}");
        } else {
            self.progress.println(line);
        }
    }
}

pub fn print_projects(
    root: &Path,
    projects: impl IntoIterator<Item = (String, String, String, PathBuf)>,
) {
    println!(
        "{} {}",
        style("Projects").cyan().bold(),
        style(root.display()).dim()
    );

    for (package, bin, target, output) in projects {
        println!(
            "  {} {} {} {} {}",
            style("•").cyan(),
            style(package).bold(),
            style(format!("[bin: {bin}]")).dim(),
            style(format!("[target: {target}]")).dim(),
            style(shorten_path(&output)).dim()
        );
    }
}

pub fn print_error(err: &Error) {
    eprintln!("{} {err:#}", style("error:").red().bold());
}

fn pluralize(count: usize, singular: &str) -> String {
    if count == 1 {
        singular.to_string()
    } else {
        format!("{singular}s")
    }
}

fn format_suffix(copied_files: usize) -> String {
    if copied_files == 0 {
        String::new()
    } else {
        format!(
            " {}",
            style(format!(
                "({} {})",
                style(copied_files).cyan().bold(),
                pluralize(copied_files, "copy")
            ))
            .dim()
        )
    }
}

fn shorten_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("-> {name}"))
        .unwrap_or_else(|| format!("-> {}", path.display()))
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if secs == 0 {
        format!("{millis}ms")
    } else if secs < 60 {
        format!("{secs}.{millis:03}s")
    } else {
        let minutes = secs / 60;
        let seconds = secs % 60;
        format!("{minutes}m {seconds}s")
    }
}

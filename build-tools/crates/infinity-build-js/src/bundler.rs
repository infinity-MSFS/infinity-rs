use crate::config::{Instrument, ModuleAlias, PackageSpec};
use crate::package::{self, EmittedPackage};
use infinity_build_core::{
    Artifact, BuildError, BuildResult, Builder, FileKind, GeneratedFile, SimpleArtifact,
    pick_primary, stat_file,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Re-imports: keep all rolldown types behind one use group so future
// API drift is a single search/replace away.
use rolldown::{
    Bundler, BundlerOptions, ChunkFilenamesOutputOption, InputItem, IsExternal, OutputFormat,
    Platform, RawMinifyOptions, ResolveOptions, SourceMapType,
};
use rolldown_common::ModuleType;
use rolldown_utils::indexmap::FxIndexMap;

#[derive(Debug, Clone, Default)]
pub struct BundleOptions {
    /// Output directory for raw bundles, relative to `project_root`.
    /// Defaults to `bundles`. Each instrument gets its own
    /// `<bundles_dir>/<instrument.name>/` subdirectory.
    pub bundles_dir: Option<PathBuf>,
    /// Minify the JS output.
    pub minify: bool,
    /// Emit sourcemaps. None = no sourcemaps (default).
    pub sourcemap: Option<SourceMapKind>,
    /// Skip the simulator-package emission step. Useful for CI smoke
    /// tests that just want to know whether the bundle compiles.
    pub skip_simulator_package: bool,
    /// Extra `process.env.<name> = <value>` substitutions. Values are
    /// JSON-encoded automatically — pass strings as plain strings, no
    /// quoting needed.
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy)]
pub enum SourceMapKind {
    Inline,
    External,
    File,
}

#[derive(Debug, Clone)]
pub struct JsBuildInput {
    pub instrument: Instrument,
    pub package: PackageSpec,
}

#[derive(Debug, Clone)]
pub struct JsArtifact {
    pub instrument_name: String,
    pub bundle_dir: PathBuf,
    pub generated: Vec<GeneratedFile>,
    pub package: Option<EmittedPackage>,
}

impl Artifact for JsArtifact {
    fn files(&self) -> &[GeneratedFile] {
        &self.generated
    }

    fn name(&self) -> &str {
        &self.instrument_name
    }

    fn primary(&self) -> Option<&GeneratedFile> {
        self.generated
            .iter()
            .find(|f| matches!(f.kind, FileKind::Template))
            .or_else(|| pick_primary(&self.generated))
    }
}

impl From<JsArtifact> for SimpleArtifact {
    fn from(value: JsArtifact) -> Self {
        SimpleArtifact::new(value.instrument_name, value.generated)
    }
}

pub struct JsBundler {
    project_root: PathBuf,
    options: BundleOptions,
}

impl JsBundler {
    pub fn new(project_root: impl Into<PathBuf>, options: BundleOptions) -> Self {
        Self {
            project_root: project_root.into(),
            options,
        }
    }

    /// Async entry point. Use this when you're already inside a tokio
    /// runtime; calling [`Builder::build`] from inside one will panic
    /// (`cannot start a runtime from within a runtime`).
    pub async fn build_async(&self, input: &JsBuildInput) -> BuildResult<JsArtifact> {
        let bundles_dir = self
            .options
            .bundles_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("bundles"));
        let abs_bundle_dir = self
            .project_root
            .join(&bundles_dir)
            .join(&input.instrument.name);
        std::fs::create_dir_all(&abs_bundle_dir).map_err(|e| BuildError::io(&abs_bundle_dir, e))?;

        let entry = input.instrument.resolved_index(&self.project_root)?;
        let bundler_options = self.bundler_options(&input.instrument, &abs_bundle_dir, &entry)?;

        let mut bundler = Bundler::new(bundler_options)
            .map_err(|e| BuildError::backend_failure("rolldown-init", format_rolldown_error(&e)))?;
        bundler.write().await.map_err(|e| {
            BuildError::backend_failure("rolldown-bundle", format_rolldown_error(&e))
        })?;

        let js_bundle_path = abs_bundle_dir.join("bundle.js");
        let css_bundle_path = abs_bundle_dir.join("bundle.css");
        let css_present = css_bundle_path.exists();

        let mut generated: Vec<GeneratedFile> = Vec::new();
        if let Ok(file) = stat_file(&js_bundle_path, FileKind::Script) {
            generated.push(file);
        }
        if css_present {
            if let Ok(file) = stat_file(&css_bundle_path, FileKind::Style) {
                generated.push(file);
            }
        }

        let package = if let Some(sim_pkg) = &input.instrument.simulator_package {
            if self.options.skip_simulator_package {
                None
            } else {
                let emitted = package::write_package(
                    &self.project_root,
                    &input.package,
                    &input.instrument,
                    sim_pkg,
                    &js_bundle_path,
                    if css_present {
                        Some(&css_bundle_path)
                    } else {
                        None
                    },
                )?;
                push_emitted_files(&emitted, &mut generated);
                Some(emitted)
            }
        } else {
            None
        };

        Ok(JsArtifact {
            instrument_name: input.instrument.name.clone(),
            bundle_dir: abs_bundle_dir,
            generated,
            package,
        })
    }

    fn bundler_options(
        &self,
        instrument: &Instrument,
        abs_bundle_dir: &Path,
        entry: &Path,
    ) -> BuildResult<BundlerOptions> {
        let mut opts = BundlerOptions::default();

        opts.input = Some(vec![InputItem {
            name: Some("bundle".to_string()),
            import: entry.to_string_lossy().into_owned(),
        }]);
        opts.cwd = Some(self.project_root.clone());
        opts.dir = Some(abs_bundle_dir.to_string_lossy().into_owned());
        opts.platform = Some(Platform::Browser);
        opts.format = Some(OutputFormat::Iife);

        // Force literal `bundle.js` / `bundle.css` filenames (no hash).
        // The "[name]" placeholder becomes our InputItem.name = "bundle".
        opts.entry_filenames = Some(ChunkFilenamesOutputOption::String("[name].js".to_string()));
        opts.css_entry_filenames =
            Some(ChunkFilenamesOutputOption::String("[name].css".to_string()));

        // Externals matching mach. Note IsExternal in 0.1.0 has a
        // `From<Vec<String>>` impl on the deserializer side; the
        // public ctor used to be `from_vec`. If the next compile says
        // otherwise, switch to whatever ctor is exposed.
        // FIXME(rolldown-0.1): confirm IsExternal constructor name.
        opts.external = Some(IsExternal::from(vec![
            "/Images/*".to_string(),
            "/Fonts/*".to_string(),
        ]));

        // Treat .otf/.ttf as assets: rolldown copies them next to the
        // bundle and rewrites imports to relative URLs. Closest match
        // to mach's `loader: { ".otf": "file" }`.
        let mut module_types: rustc_hash::FxHashMap<String, ModuleType> = Default::default();
        module_types.insert(".otf".to_string(), ModuleType::Asset);
        module_types.insert(".ttf".to_string(), ModuleType::Asset);
        opts.module_types = Some(module_types);

        // JSX classic React runtime is the default in rolldown 0.1.0
        // (JsxPreset::Enable). Leaving `opts.transform` unset gives us
        // that out of the box, matching the mach default.

        // Module aliases for nested instruments (mach's `modules`
        // feature). Rolldown's ResolveOptions.alias takes a list of
        // (specifier, replacement-paths) tuples.
        // FIXME(rolldown-0.1): confirm whether alias is `Vec<(String, Vec<String>)>`
        // or `IndexMap<String, Vec<String>>`. Both shapes appear in
        // the wider rolldown ecosystem.
        if !instrument.modules.is_empty() {
            let mut resolve = ResolveOptions::default();
            let alias_entries: Vec<(String, Vec<Option<String>>)> = instrument
                .modules
                .iter()
                .map(|ModuleAlias { resolve, index }| {
                    let abs = self.project_root.join(index);
                    (
                        resolve.clone(),
                        vec![Some(abs.to_string_lossy().into_owned())],
                    )
                })
                .collect();
            resolve.alias = Some(alias_entries);
            opts.resolve = Some(resolve);
        }

        // process.env.* substitutions via `define`. Values are JSON-
        // encoded so quoting is preserved — much cleaner than mach's
        // regex hack. In 0.1.0 `define` is `Option<FxIndexMap<String,
        // String>>`. We use the rustc_hash + indexmap re-export
        // surface that rolldown re-exports.
        if !self.options.env.is_empty() {
            let mut define: FxIndexMap<String, String> = Default::default();
            for (key, value) in &self.options.env {
                let json_value = serde_json::to_string(value).unwrap_or_else(|_| "null".into());
                define.insert(format!("process.env.{key}"), json_value);
            }
            opts.define = Some(define);
        }

        if self.options.minify {
            opts.minify = Some(RawMinifyOptions::Bool(true));
        }

        if let Some(kind) = self.options.sourcemap {
            // rolldown 0.1.0's SourceMapType is {File, Inline, Hidden}.
            // We map "external" (linked) → File (writes .map + adds the
            // sourceMappingURL comment) and "file" → Hidden (writes
            // .map without the comment, for shipping side-by-side
            // without exposing a link).
            opts.sourcemap = Some(match kind {
                SourceMapKind::Inline => SourceMapType::Inline,
                SourceMapKind::External => SourceMapType::File,
                SourceMapKind::File => SourceMapType::Hidden,
            });
        }

        Ok(opts)
    }
}

/// `Builder` impl. Boots a current-thread tokio runtime per call. If
/// you already have a runtime, prefer [`JsBundler::build_async`].
impl Builder for JsBundler {
    type Input = JsBuildInput;
    type Output = JsArtifact;

    fn build(&self, input: &Self::Input) -> BuildResult<Self::Output> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                BuildError::backend_failure(
                    "tokio-runtime",
                    format!("could not start runtime: {e}"),
                )
            })?;
        rt.block_on(self.build_async(input))
    }
}

fn push_emitted_files(emitted: &EmittedPackage, into: &mut Vec<GeneratedFile>) {
    for path in emitted.iter_paths() {
        let kind = match path.extension().and_then(|e| e.to_str()) {
            Some("html") => FileKind::Template,
            Some("css") => FileKind::Style,
            Some("js" | "mjs" | "cjs") => FileKind::Script,
            Some("map") => FileKind::SourceMap,
            _ => FileKind::Other,
        };
        if let Ok(file) = stat_file(path, kind) {
            into.push(file);
        }
    }
}

/// Render a rolldown error into a single human-readable string. The
/// rolldown error type's `Display` is often empty for diagnostic
/// containers, so we fall back to `Debug` if Display gives us nothing.
fn format_rolldown_error<E: std::fmt::Debug + std::fmt::Display>(err: &E) -> String {
    let display = format!("{err}");
    if display.trim().is_empty() {
        format!("{err:?}")
    } else {
        display
    }
}

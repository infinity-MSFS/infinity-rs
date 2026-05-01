use infinity_build_core::{BuildError, BuildResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsBuildConfig {
    #[serde(flatten)]
    pub package: PackageSpec,

    #[serde(default)]
    pub instruments: Vec<Instrument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageSpec {
    pub package_name: String,

    #[serde(default = "default_package_dir")]
    pub package_dir: PathBuf,
}

fn default_package_dir() -> PathBuf {
    PathBuf::from("PackageSources")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Instrument {
    pub name: String,
    pub index: PathBuf,

    #[serde(default)]
    pub simulator_package: Option<SimulatorPackage>,

    #[serde(default)]
    pub modules: Vec<ModuleAlias>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModuleAlias {
    pub resolve: String,
    pub index: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SimulatorPackage {
    React {
        #[serde(default)]
        file_name: Option<String>,

        #[serde(default)]
        template_id: Option<String>,

        #[serde(default = "default_true")]
        is_interactive: bool,

        #[serde(default)]
        imports: Vec<String>,

        #[serde(default)]
        html_template: Option<PathBuf>,

        #[serde(default)]
        js_template: Option<PathBuf>,
    },

    BaseInstrument {
        #[serde(default)]
        file_name: Option<String>,

        /// Required. Must match `BaseInstrument.templateID()`.
        template_id: String,
        /// Required. Must match the ID passed to `FSComponent.render()`.
        mount_element_id: String,

        #[serde(default)]
        imports: Vec<String>,

        #[serde(default)]
        html_template: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulatorPackageKind {
    React,
    BaseInstrument,
}

fn default_true() -> bool {
    true
}

impl SimulatorPackage {
    pub fn kind(&self) -> SimulatorPackageKind {
        match self {
            SimulatorPackage::React { .. } => SimulatorPackageKind::React,
            SimulatorPackage::BaseInstrument { .. } => SimulatorPackageKind::BaseInstrument,
        }
    }

    pub fn file_name(&self) -> &str {
        match self {
            SimulatorPackage::React { file_name, .. }
            | SimulatorPackage::BaseInstrument { file_name, .. } => {
                file_name.as_deref().unwrap_or("instrument")
            }
        }
    }

    pub fn imports(&self) -> &[String] {
        match self {
            SimulatorPackage::React { imports, .. }
            | SimulatorPackage::BaseInstrument { imports, .. } => imports,
        }
    }
}

/// Strip the Windows `\\?\` verbatim prefix from a canonicalized path.
/// Rolldown/oxc_resolver treats verbatim paths as unresolvable URL-like
/// strings (`//?/C:/...`), so we hand them plain drive-letter paths.
#[cfg(windows)]
fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    let s = path.as_os_str().to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        // Keep UNC shares as `\\server\share\...`
        if let Some(unc) = rest.strip_prefix(r"UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        return PathBuf::from(rest.to_string());
    }
    path
}

#[cfg(not(windows))]
fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    path
}

impl Instrument {
    pub fn resolved_index(&self, project_root: &Path) -> BuildResult<PathBuf> {
        let abs = project_root.join(&self.index);
        let canonical = std::fs::canonicalize(&abs)
            .map_err(|e| BuildError::invalid_path(abs, format!("entrypoint not found: {e}")))?;
        Ok(strip_verbatim_prefix(canonical))
    }
}

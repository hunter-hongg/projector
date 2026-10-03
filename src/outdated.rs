//! Cross-ecosystem "is there a newer version?" checks.
//!
//! Projector is a local analysis tool: it never talks to a registry on its
//! own. [`check_project`] is reached only when the user passes `--outdated`,
//! and even then it delegates to the ecosystem's *own* tooling rather than
//! reimplementing a crates.io / npm / PyPI / module-proxy client here.
//!
//! Each ecosystem therefore has three parts:
//! - a pure `parse_*` function over the tool's stdout (unit-testable offline),
//! - a [`Probe`] naming the tool and how to install it,
//! - the [`run`] helper that executes it and maps the outcome to [`Outcome`].
//!
//! A missing tool is not an error: it becomes [`Outcome::ToolMissing`] with an
//! install hint, so `deps --outdated` stays useful on a machine where only
//! some toolchains are installed.

use std::path::Path;
use std::process::Command;

/// One dependency whose upstream version has moved past what is declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutdatedEntry {
    pub name: String,
    /// Version currently resolved/installed in the project.
    pub current: String,
    /// Version that satisfies the declared requirement.
    pub wanted: String,
    /// Latest version published upstream.
    pub latest: String,
    /// Ecosystem label matching `DependencyEntry::dep_type` ("rust", "js", ...).
    pub dep_type: String,
    /// Project this was found in.
    pub project_path: String,
}

/// What checking one ecosystem in one project produced.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// Tool ran; the entries are the outdated ones (possibly empty).
    Found(Vec<OutdatedEntry>),
    /// Tool binary is not on `PATH` — [`Probe::install_hint`] says how to get it.
    ToolMissing(Probe),
    /// Tool ran but failed, or its output could not be read.
    Failed { probe: Probe, message: String },
}

impl Outcome {
    fn found(&self) -> Option<&[OutdatedEntry]> {
        match self {
            Outcome::Found(entries) => Some(entries),
            _ => None,
        }
    }
}

/// The external tool one ecosystem is checked with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    CargoOutdated,
    NpmOutdated,
    GoList,
    PipList,
}

impl Probe {
    /// Program name as the user would type it.
    pub fn binary(&self) -> &'static str {
        match self {
            Probe::CargoOutdated => "cargo-outdated",
            Probe::NpmOutdated => "npm",
            Probe::GoList => "go",
            Probe::PipList => "pip",
        }
    }

    pub fn install_hint(&self) -> &'static str {
        match self {
            Probe::CargoOutdated => "install it with `cargo install cargo-outdated`",
            Probe::NpmOutdated => "install Node.js/npm (https://nodejs.org)",
            Probe::GoList => "install the Go toolchain (https://go.dev/dl)",
            Probe::PipList => "install pip (`python -m ensurepip --upgrade`)",
        }
    }

    /// Which probe applies to an ecosystem, if any supports an outdated check.
    fn for_dep_type(dep_type: &str) -> Option<Probe> {
        match dep_type {
            "rust" => Some(Probe::CargoOutdated),
            "js" => Some(Probe::NpmOutdated),
            "go" => Some(Probe::GoList),
            "python" => Some(Probe::PipList),
            _ => None,
        }
    }

    /// Ecosystem labels this probe reports for, used to keep one probe per
    /// project even when a project has several manifests (e.g. pyproject +
    /// requirements.txt).
    fn dep_types(&self) -> &'static [&'static str] {
        match self {
            Probe::CargoOutdated => &["rust"],
            Probe::NpmOutdated => &["js"],
            Probe::GoList => &["go"],
            Probe::PipList => &["python"],
        }
    }
}

/// A manifest file that makes an outdated check meaningful for `dep_type`.
fn manifest_for(dep_type: &str, dir: &Path) -> Option<std::path::PathBuf> {
    let candidates: &[&str] = match dep_type {
        "rust" => &["Cargo.toml"],
        "js" => &["package.json"],
        "go" => &["go.mod"],
        "python" => &["pyproject.toml", "requirements.txt"],
        _ => &[],
    };
    candidates
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

/// True when the probe's tool is usable.
///
/// `cargo outdated` is a third-party *subcommand*, so probing an executable
/// name would mean guessing (`cargo-outdated`? `cargo outdated`?). Asking
/// `cargo` itself is simpler and portable: without the subcommand installed
/// cargo exits non-zero, which is exactly the "install it first" case.
fn probe_available(probe: Probe) -> bool {
    let (program, args) = match probe {
        Probe::CargoOutdated => ("cargo", vec!["outdated", "--help"]),
        Probe::NpmOutdated => ("npm", vec!["--version"]),
        Probe::GoList => ("go", vec!["version"]),
        Probe::PipList => ("pip", vec!["--version"]),
    };
    Command::new(program)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Run the probe's tool in `dir` and parse its output.
fn run(probe: Probe, dir: &Path, project_path: &str) -> Outcome {
    if !probe_available(probe) {
        return Outcome::ToolMissing(probe);
    }

    let (program, args) = match probe {
        Probe::CargoOutdated => ("cargo", vec!["outdated", "--workspace", "--format", "json"]),
        Probe::NpmOutdated => ("npm", vec!["outdated", "--json"]),
        Probe::GoList => ("go", vec!["list", "-m", "-u", "all"]),
        Probe::PipList => ("pip", vec!["list", "--outdated", "--format", "json"]),
    };

    let output = match Command::new(program).args(&args).current_dir(dir).output() {
        Ok(o) => o,
        Err(e) => {
            return Outcome::Failed {
                probe,
                message: format!("could not run `{} {}`: {}", program, args.join(" "), e),
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let parsed = match probe {
        Probe::CargoOutdated => parse_cargo_outdated(&stdout, project_path),
        Probe::NpmOutdated => parse_npm_outdated(&stdout, project_path),
        Probe::GoList => parse_go_outdated(&stdout, project_path),
        Probe::PipList => parse_pip_outdated(&stdout, project_path),
    };

    match parsed {
        Some(entries) => Outcome::Found(entries),
        None => {
            // Every one of these tools exits non-zero when it has findings
            // (npm exits 1, cargo-outdated exits 1), so a non-zero status with
            // unparseable output is a real failure and gets reported verbatim.
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                format!(
                    "`{} {}` exited {} with unreadable output",
                    program,
                    args.join(" "),
                    output.status.code().unwrap_or(-1)
                )
            } else {
                stderr.lines().next().unwrap_or_default().to_string()
            };
            Outcome::Failed { probe, message }
        }
    }
}

/// Check every ecosystem present in `dir` that projector has deps for.
///
/// Only ecosystems found in `dep_types` are probed, so a Go project never
/// shells out to npm. Returns one [`Outcome`] per probed ecosystem.
pub fn check_project(dir: &Path, project_path: &str, dep_types: &[&str]) -> Vec<Outcome> {
    let mut outcomes = Vec::new();
    let mut probed: Vec<Probe> = Vec::new();

    for dep_type in dep_types {
        if manifest_for(dep_type, dir).is_none() {
            continue;
        }
        let Some(probe) = Probe::for_dep_type(dep_type) else {
            continue;
        };
        if probed.contains(&probe) {
            continue;
        }
        probed.push(probe);
        outcomes.push(run(probe, dir, project_path));
    }

    outcomes
}

/// Collect the entries out of a set of outcomes.
pub fn entries(outcomes: &[Outcome]) -> Vec<OutdatedEntry> {
    outcomes
        .iter()
        .flat_map(|o| o.found().unwrap_or(&[]).to_vec())
        .collect()
}

/// What a whole `--outdated` run found across a set of projects.
#[derive(Debug, Default)]
pub struct Report {
    pub entries: Vec<OutdatedEntry>,
    /// Projects actually probed (those with a checkable manifest on disk).
    pub projects_checked: usize,
    /// Tools the user needs to install to cover the rest.
    pub missing: Vec<Probe>,
    /// Tools that ran and failed, with their first stderr line.
    pub failures: Vec<(Probe, String)>,
}

/// Check every project represented in `deps`.
///
/// `deps` is whatever `deps` already resolved (one project, or every project in
/// the latest snapshot), so `--project` filtering and path arguments apply to
/// `--outdated` for free.
pub fn collect(deps: &[crate::dependencies::DependencyEntry]) -> Report {
    let mut report = Report::default();

    let mut projects: Vec<&String> = deps.iter().map(|d| &d.project_path).collect();
    projects.sort();
    projects.dedup();

    for project_path in projects {
        let dir = Path::new(project_path);
        if !dir.exists() {
            continue;
        }
        let project_deps: Vec<&crate::dependencies::DependencyEntry> = deps
            .iter()
            .filter(|d| &d.project_path == project_path)
            .collect();
        let dep_types = dep_types_present_owned(&project_deps);
        if dep_types.is_empty() {
            continue;
        }

        let refs: Vec<&str> = dep_types.to_vec();
        let outcomes = check_project(dir, project_path, &refs);
        if outcomes.is_empty() {
            continue;
        }
        report.projects_checked += 1;
        report.entries.extend(entries(&outcomes));
        for probe in missing_probes(&outcomes) {
            if !report.missing.contains(&probe) {
                report.missing.push(probe);
            }
        }
        for failure in failure_messages(&outcomes) {
            if !report.failures.contains(&failure) {
                report.failures.push(failure);
            }
        }
    }

    report
}

/// [`dep_types_present`] over borrowed entries.
fn dep_types_present_owned(deps: &[&crate::dependencies::DependencyEntry]) -> Vec<&'static str> {
    let mut types: Vec<&'static str> = Vec::new();
    for d in deps {
        for t in dep_types_present(std::slice::from_ref(d)) {
            if !types.contains(&t) {
                types.push(t);
            }
        }
    }
    types
}

/// The tool that is not installed, if any outcome is [`Outcome::ToolMissing`].
pub fn missing_probes(outcomes: &[Outcome]) -> Vec<Probe> {
    outcomes
        .iter()
        .filter_map(|o| match o {
            Outcome::ToolMissing(p) => Some(*p),
            _ => None,
        })
        .collect()
}

/// Human-readable reason for an outcome that produced no data.
pub fn failure_messages(outcomes: &[Outcome]) -> Vec<(Probe, String)> {
    outcomes
        .iter()
        .filter_map(|o| match o {
            Outcome::Failed { probe, message } => Some((*probe, message.clone())),
            _ => None,
        })
        .collect()
}

/// `cargo outdated --format json` emits one object per outdated crate:
/// `{"name":..,"project":..,"name":..,"pkg":..,"old":..,"new":..,"wanted":..}`.
pub fn parse_cargo_outdated(json: &str, project_path: &str) -> Option<Vec<OutdatedEntry>> {
    let value: serde_json::Value = serde_json::from_str(json.trim()).ok()?;
    let rows = match &value {
        serde_json::Value::Array(rows) => rows.clone(),
        // Some versions wrap the list under "crates".
        serde_json::Value::Object(map) => map
            .get("crates")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => return Some(Vec::new()),
    };

    let entries = rows
        .iter()
        .filter_map(|row| {
            Some(OutdatedEntry {
                name: row.get("name")?.as_str()?.to_string(),
                current: known_field(row, "pkg"),
                wanted: known_field(row, "wanted"),
                latest: known_field(row, "new"),
                dep_type: "rust".to_string(),
                project_path: project_path.to_string(),
            })
        })
        .collect();
    Some(entries)
}

/// `npm outdated --json` emits `{"pkg":{"current":"1.0.0","wanted":"1.0.1","latest":"2.0.0",..}}`,
/// and `{}` (or empty output) when everything is current.
///
/// npm omits `current` when the package is not installed on disk (no
/// `node_modules`), so that case reads as [`NOT_INSTALLED`] rather than a
/// version — and never as a fake "all current".
pub fn parse_npm_outdated(json: &str, project_path: &str) -> Option<Vec<OutdatedEntry>> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let map = value.as_object()?;

    let mut entries: Vec<OutdatedEntry> = map
        .iter()
        .map(|(name, info)| OutdatedEntry {
            name: name.clone(),
            current: string_field(info, "current").unwrap_or_else(|| NOT_INSTALLED.to_string()),
            wanted: string_field(info, "wanted").unwrap_or_default(),
            latest: string_field(info, "latest").unwrap_or_default(),
            dep_type: "js".to_string(),
            project_path: project_path.to_string(),
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Some(entries)
}

/// Placeholder for "declared but not present on disk", which npm and pip both
/// report by simply omitting the installed version.
const NOT_INSTALLED: &str = "not installed";

/// `pip list --outdated --format json` emits
/// `[{"name":"requests","version":"1.0","latest_version":"2.0","latest_filetype":"wheel"}]`.
pub fn parse_pip_outdated(json: &str, project_path: &str) -> Option<Vec<OutdatedEntry>> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let rows = value.as_array()?.clone();

    let entries = rows
        .iter()
        .filter_map(|row| {
            Some(OutdatedEntry {
                name: row.get("name")?.as_str()?.to_string(),
                current: string_field(row, "version").unwrap_or_else(|| NOT_INSTALLED.to_string()),
                wanted: string_field(row, "version").unwrap_or_default(),
                latest: string_field(row, "latest_version").unwrap_or_default(),
                dep_type: "python".to_string(),
                project_path: project_path.to_string(),
            })
        })
        .collect();
    Some(entries)
}

/// `go list -m -u all` lists every module, marking a newer release inline:
///
/// ```text
/// github.com/x/y v1.0.0 [v1.1.0]
/// github.com/x/z v2.1.0
/// ```
///
/// Only the bracketed lines are outdated; the bracket holds the latest known
/// release. `+incompatible` and `v0.0.0-` pseudo-version noise are kept as-is
/// since they are what the module actually resolves to.
pub fn parse_go_outdated(stdout: &str, project_path: &str) -> Option<Vec<OutdatedEntry>> {
    let mut entries = Vec::new();

    for line in stdout.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let Some(open) = line.rfind('[') else {
            continue;
        };
        let Some(close) = line.rfind(']') else {
            continue;
        };
        if close < open {
            continue;
        }
        let latest = line[open + 1..close].trim().to_string();
        let rest = line[..open].trim();
        let mut parts = rest.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(current) = parts.next() else {
            continue;
        };

        entries.push(OutdatedEntry {
            name: name.to_string(),
            current: current.to_string(),
            wanted: current.to_string(),
            latest,
            dep_type: "go".to_string(),
            project_path: project_path.to_string(),
        });
    }

    Some(entries)
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// [`string_field`] with an explicit "unknown" fallback, for the fields
/// cargo-outdated always fills.
fn known_field(value: &serde_json::Value, key: &str) -> String {
    string_field(value, key).unwrap_or_else(|| "unknown".to_string())
}

/// Ecosystem labels a project's parsed dependencies can be checked in, deduped
/// and ordered by first appearance, as `dep_type` strings.
///
/// Derived from what the manifest parser actually found, so an ecosystem is
/// never probed (and never reported as "tool missing") unless the project
/// really declares dependencies in it.
pub fn dep_types_present(deps: &[crate::dependencies::DependencyEntry]) -> Vec<&'static str> {
    let mut types: Vec<&str> = Vec::new();
    for d in deps {
        let t: &'static str = match d.dep_type.as_str() {
            "rust" => "rust",
            "js" => "js",
            "go" => "go",
            "python" => "python",
            _ => continue,
        };
        if !types.contains(&t) {
            types.push(t);
        }
    }
    types
}

impl Probe {
    /// Ecosystem label this probe covers, for JSON output.
    pub fn dep_type(&self) -> &'static str {
        self.dep_types()[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARGO_FIXTURE: &str = r#"[
      {"name":"serde","project":"app","std":"serde","pkg":"1.0.190","old":"1.0.190","wanted":"1.0.190","new":"1.0.219","ignore":false,"project_repo":null},
      {"name":"tokio","project":"app","std":"tokio","pkg":"1.35.0","old":"1.35.0","wanted":"1.35.0","new":"1.41.0","ignore":false,"project_repo":null}
    ]"#;

    #[test]
    fn cargo_fixture_yields_one_entry_per_crate() {
        let entries = parse_cargo_outdated(CARGO_FIXTURE, "/proj/app").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "serde");
        assert_eq!(entries[0].current, "1.0.190");
        assert_eq!(entries[0].latest, "1.0.219");
        assert_eq!(entries[0].dep_type, "rust");
        assert_eq!(entries[0].project_path, "/proj/app");
    }

    #[test]
    fn cargo_empty_array_is_not_an_error() {
        let entries = parse_cargo_outdated("[]", "/proj/app").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn cargo_wrapped_crates_object_is_understood() {
        let json = r#"{"crates":[{"name":"log","pkg":"0.4.0","wanted":"0.4.0","new":"0.4.22"}]}"#;
        let entries = parse_cargo_outdated(json, "/p").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].latest, "0.4.22");
    }

    #[test]
    fn cargo_garbage_reports_none_not_empty() {
        // Distinguishing "nothing outdated" from "could not read the tool" is
        // what lets the CLI show a real error instead of a false all-clear.
        assert!(parse_cargo_outdated("error: no such command", "/p").is_none());
    }

    const NPM_FIXTURE: &str = r#"{
      "lodash": {"current":"4.17.20","wanted":"4.17.20","latest":"4.17.21","dependent":"^4.17.20","location":"node_modules/lodash"},
      "chalk": {"current":"4.1.0","wanted":"4.1.2","latest":"5.3.0","dependent":"^4.1.0","location":"node_modules/chalk"}
    }"#;

    #[test]
    fn npm_fixture_is_sorted_by_name() {
        let entries = parse_npm_outdated(NPM_FIXTURE, "/proj/web").unwrap();
        assert_eq!(
            entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            vec!["chalk", "lodash"]
        );
        assert_eq!(entries[0].wanted, "4.1.2");
        assert_eq!(entries[0].dep_type, "js");
    }

    #[test]
    fn npm_empty_object_and_empty_output_both_mean_current() {
        assert!(parse_npm_outdated("{}", "/p").unwrap().is_empty());
        assert!(parse_npm_outdated("", "/p").unwrap().is_empty());
        assert!(parse_npm_outdated("   \n", "/p").unwrap().is_empty());
    }

    #[test]
    fn npm_non_object_output_reports_none() {
        assert!(parse_npm_outdated("npm ERR! code ELAYOUT", "/p").is_none());
    }

    #[test]
    fn npm_without_node_modules_says_not_installed() {
        // Verbatim shape of `npm outdated --json` in a project with no
        // node_modules: npm reports what it would want and omits `current`.
        let json = r#"{"lodash":{"wanted":"4.18.1","latest":"4.18.1","dependent":"app"}}"#;
        let entries = parse_npm_outdated(json, "/p").unwrap();
        assert_eq!(entries[0].current, "not installed");
        assert_eq!(entries[0].latest, "4.18.1");
    }

    #[test]
    fn pip_fixture_reads_latest_version() {
        let json = r#"[{"name":"requests","version":"2.19.0","latest_version":"2.32.3","latest_filetype":"wheel"}]"#;
        let entries = parse_pip_outdated(json, "/proj/svc").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].current, "2.19.0");
        assert_eq!(entries[0].latest, "2.32.3");
        assert_eq!(entries[0].dep_type, "python");
    }

    #[test]
    fn pip_empty_list_is_current() {
        assert!(parse_pip_outdated("[]", "/p").unwrap().is_empty());
        assert!(parse_pip_outdated("", "/p").unwrap().is_empty());
    }

    #[test]
    fn go_only_bracketed_modules_are_outdated() {
        let stdout = "\
github.com/a/cur v1.0.0
github.com/b/old v1.0.0 [v1.1.0]
github.com/c/pseudo v0.0.0-20210101000000-abcdef123456 [v0.1.0]
";
        let entries = parse_go_outdated(stdout, "/proj/svc").unwrap();
        assert_eq!(entries.len(), 2, "unbracketed modules are current");
        assert_eq!(entries[0].name, "github.com/b/old");
        assert_eq!(entries[0].current, "v1.0.0");
        assert_eq!(entries[0].latest, "v1.1.0");
        assert_eq!(entries[0].dep_type, "go");
        assert_eq!(entries[1].latest, "v0.1.0");
    }

    #[test]
    fn go_empty_output_is_current() {
        assert!(parse_go_outdated("", "/p").unwrap().is_empty());
        assert!(parse_go_outdated("\n\n", "/p").unwrap().is_empty());
    }

    #[test]
    fn malformed_go_lines_are_skipped() {
        let entries = parse_go_outdated("garbage [v1.0.0\n", "/p").unwrap();
        assert!(
            entries.is_empty(),
            "a module line needs a name and a version"
        );
    }

    #[test]
    fn every_probe_has_an_actionable_install_hint() {
        for probe in [
            Probe::CargoOutdated,
            Probe::NpmOutdated,
            Probe::GoList,
            Probe::PipList,
        ] {
            assert!(!probe.binary().is_empty());
            assert!(
                probe.install_hint().contains(' ') || probe.install_hint().contains('`'),
                "{probe:?} hint is not actionable"
            );
            assert!(!probe.dep_type().is_empty());
        }
    }

    #[test]
    fn probe_maps_only_supported_ecosystems() {
        assert_eq!(Probe::for_dep_type("rust"), Some(Probe::CargoOutdated));
        assert_eq!(Probe::for_dep_type("js"), Some(Probe::NpmOutdated));
        assert_eq!(Probe::for_dep_type("go"), Some(Probe::GoList));
        assert_eq!(Probe::for_dep_type("python"), Some(Probe::PipList));
        assert_eq!(Probe::for_dep_type("unknown"), None);
    }

    #[test]
    fn dep_types_present_reads_parsed_entries() {
        let dep = |dep_type: &str| crate::dependencies::DependencyEntry {
            name: "x".into(),
            version_req: "1".into(),
            project_path: "/p".into(),
            dep_type: dep_type.into(),
            is_dev: false,
        };
        let deps = vec![dep("rust"), dep("rust"), dep("python"), dep("unknown")];
        assert_eq!(dep_types_present(&deps), vec!["rust", "python"]);
        assert!(dep_types_present(&[]).is_empty());
        assert!(dep_types_present(&[dep("unknown")]).is_empty());
    }

    #[test]
    fn every_supported_dep_type_has_a_probe() {
        for label in ["rust", "js", "go", "python"] {
            assert!(
                Probe::for_dep_type(label).is_some(),
                "dep_type {label} is mapped by dep_types_present but has no probe"
            );
        }
    }

    #[test]
    fn check_project_probes_only_manifests_that_exist() {
        let dir = std::env::temp_dir().join(format!("projector_outdated_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // No manifests at all: nothing is probed, so no tool is ever run.
        assert!(check_project(&dir, &dir.to_string_lossy(), &["rust", "js"]).is_empty());

        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        // Two python manifests must still yield a single python probe.
        std::fs::write(dir.join("requirements.txt"), "requests>=2.0\n").unwrap();
        std::fs::write(dir.join("pyproject.toml"), "[project]\nname=\"x\"\n").unwrap();

        let outcomes = check_project(&dir, &dir.to_string_lossy(), &["rust", "python", "go"]);
        let python_probes = outcomes
            .iter()
            .filter(|o| match o {
                Outcome::ToolMissing(p) | Outcome::Failed { probe: p, .. } => *p == Probe::PipList,
                Outcome::Found(e) => e.iter().any(|d| d.dep_type == "python"),
            })
            .count();
        assert!(python_probes <= 1, "duplicate python probe: {outcomes:?}");
        // Go was requested but has no go.mod, so no Go outcome may appear.
        assert!(
            !outcomes.iter().any(|o| match o {
                Outcome::ToolMissing(p) | Outcome::Failed { probe: p, .. } => *p == Probe::GoList,
                Outcome::Found(e) => e.iter().any(|d| d.dep_type == "go"),
            }),
            "go must not be probed without go.mod: {outcomes:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn collect_skips_projects_without_manifests_or_deps() {
        // A project path that does not exist must never shell out.
        let deps = vec![crate::dependencies::DependencyEntry {
            name: "serde".into(),
            version_req: "1.0".into(),
            project_path: "/nonexistent/projector/outdated/probe".into(),
            dep_type: "rust".into(),
            is_dev: false,
        }];
        let report = collect(&deps);
        assert_eq!(report.projects_checked, 0);
        assert!(report.entries.is_empty());
        assert!(report.missing.is_empty());
    }

    #[test]
    fn collect_counts_one_project_per_existing_manifest() {
        let dir =
            std::env::temp_dir().join(format!("projector_outdated_collect_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[dependencies]\nserde=\"1\"\n").unwrap();

        let report = collect(&[crate::dependencies::DependencyEntry {
            name: "serde".into(),
            version_req: "1.0".into(),
            project_path: dir.to_string_lossy().to_string(),
            dep_type: "rust".into(),
            is_dev: false,
        }]);

        // Whether cargo-outdated is installed only changes *how* the project is
        // accounted for, never *whether* it was attempted.
        assert_eq!(report.projects_checked, 1, "{report:?}");
        assert!(
            report.missing.is_empty() || report.entries.is_empty(),
            "a missing tool must not also claim results: {report:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entries_and_missing_are_extracted_per_outcome() {
        let outcomes = vec![
            Outcome::Found(vec![OutdatedEntry {
                name: "serde".into(),
                current: "1.0".into(),
                wanted: "1.0".into(),
                latest: "1.1".into(),
                dep_type: "rust".into(),
                project_path: "/p".into(),
            }]),
            Outcome::ToolMissing(Probe::NpmOutdated),
            Outcome::Failed {
                probe: Probe::GoList,
                message: "go: connection refused".into(),
            },
        ];
        assert_eq!(entries(&outcomes).len(), 1);
        assert_eq!(missing_probes(&outcomes), vec![Probe::NpmOutdated]);
        assert_eq!(failure_messages(&outcomes).len(), 1);
    }
}

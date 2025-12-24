//! Reverse dependency analysis for removal preflight checks.
//!
//! This module provides functionality to analyze reverse dependencies, finding all packages
//! that depend on packages being removed. It uses breadth-first search (BFS) traversal
//! with `pacman -Qi` queries to build a complete dependency graph.

use crate::deps::query::get_installed_packages;
use crate::error::{ArchToolkitError, Result};
use crate::types::dependency::{
    Dependency, DependencySource, DependencyStatus, PackageRef, ReverseDependencyReport,
    ReverseDependencySummary,
};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque, hash_map::Entry};
use std::process::{Command, Stdio};

/// Reverse dependency analyzer for removal operations.
///
/// This struct provides the main entry point for analyzing reverse dependencies
/// for packages being removed. It performs BFS traversal to find all packages
/// that depend on the removal targets.
pub struct ReverseDependencyAnalyzer;

impl ReverseDependencyAnalyzer {
    /// What: Create a new reverse dependency analyzer.
    ///
    /// Inputs:
    /// - (none)
    ///
    /// Output:
    /// - Returns a new `ReverseDependencyAnalyzer` instance.
    ///
    /// Details:
    /// - Creates an analyzer with default configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use arch_toolkit::deps::ReverseDependencyAnalyzer;
    ///
    /// let analyzer = ReverseDependencyAnalyzer::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// What: Analyze reverse dependencies for packages being removed.
    ///
    /// Inputs:
    /// - `packages`: A slice of `PackageRef` instances for packages being removed.
    ///
    /// Output:
    /// - Returns a `Result` containing `ReverseDependencyReport` on success, or an `ArchToolkitError` on failure.
    ///
    /// Details:
    /// - Performs breadth-first search (BFS) traversal using `pacman -Qi` metadata.
    /// - Aggregates per-root relationships to track direct vs transitive dependents.
    /// - Only analyzes installed packages (skips uninstalled packages).
    /// - Returns empty report if no packages provided or all packages are uninstalled.
    ///
    /// # Errors
    ///
    /// This function can return an `ArchToolkitError` if underlying `pacman` commands fail
    /// or if parsing their output encounters unexpected formats.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use arch_toolkit::deps::ReverseDependencyAnalyzer;
    /// use arch_toolkit::{PackageRef, PackageSource};
    ///
    /// let analyzer = ReverseDependencyAnalyzer::new();
    /// let packages = vec![
    ///     PackageRef {
    ///         name: "qt5-base".into(),
    ///         version: "5.15.10".into(),
    ///         source: PackageSource::Official {
    ///             repo: "extra".into(),
    ///             arch: "x86_64".into(),
    ///         },
    ///     },
    /// ];
    ///
    /// let report = analyzer.analyze(&packages)?;
    /// println!("{} packages would be affected", report.dependents.len());
    /// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
    /// ```
    pub fn analyze(&self, packages: &[PackageRef]) -> Result<ReverseDependencyReport> {
        tracing::info!(
            "Starting reverse dependency resolution for {} target(s)",
            packages.len()
        );

        if packages.is_empty() {
            return Ok(ReverseDependencyReport::default());
        }

        let mut state = ReverseResolverState::new(packages);

        for target in packages {
            let root = target.name.trim();
            if root.is_empty() {
                continue;
            }

            if state.pkg_info(root).is_none() {
                tracing::warn!(
                    "Skipping reverse dependency walk for {} (not installed)",
                    root
                );
                continue;
            }

            let mut visited: HashSet<String> = HashSet::new();
            visited.insert(root.to_string());

            let mut queue: VecDeque<(String, usize)> = VecDeque::new();
            queue.push_back((root.to_string(), 0));

            while let Some((current, depth)) = queue.pop_front() {
                let Some(info) = state.pkg_info(&current) else {
                    continue;
                };

                for dependent in info.required_by.iter().filter(|name| !name.is_empty()) {
                    state.update_entry(dependent, &current, root, depth + 1);

                    if visited.insert(dependent.clone()) {
                        queue.push_back((dependent.clone(), depth + 1));
                    }
                }
            }
        }

        let ReverseResolverState { aggregated, .. } = state;

        let mut summary_map: HashMap<String, ReverseDependencySummary> = HashMap::new();
        for entry in aggregated.values() {
            for (root, relation) in &entry.per_root {
                let summary =
                    summary_map
                        .entry(root.clone())
                        .or_insert_with(|| ReverseDependencySummary {
                            package: root.clone(),
                            ..Default::default()
                        });

                if relation.parents.contains(root) || relation.min_depth() == 1 {
                    summary.direct_dependents += 1;
                } else {
                    summary.transitive_dependents += 1;
                }
                summary.total_dependents =
                    summary.direct_dependents + summary.transitive_dependents;
            }
        }

        for target in packages {
            summary_map
                .entry(target.name.clone())
                .or_insert_with(|| ReverseDependencySummary {
                    package: target.name.clone(),
                    ..Default::default()
                });
        }

        let mut summaries: Vec<ReverseDependencySummary> = summary_map.into_values().collect();
        summaries.sort_by(|a, b| a.package.cmp(&b.package));

        let mut dependencies: Vec<Dependency> = aggregated
            .into_iter()
            .map(|(name, entry)| convert_entry(name, entry))
            .collect();
        dependencies.sort_by(|a, b| a.name.cmp(&b.name));

        tracing::info!(
            "Reverse dependency resolution complete ({} impacted packages)",
            dependencies.len()
        );

        Ok(ReverseDependencyReport {
            dependents: dependencies,
            summaries,
        })
    }
}

impl Default for ReverseDependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// What: Internal working state used while traversing reverse dependencies.
///
/// Inputs:
/// - Constructed from user-selected removal targets and lazily populated with pacman metadata.
///
/// Output:
/// - Retains cached package information, aggregation maps, and bookkeeping sets during traversal.
///
/// Details:
/// - Encapsulates shared collections so helper methods can mutate state without leaking implementation details.
struct ReverseResolverState {
    /// Aggregated reverse dependency entries by package name.
    aggregated: HashMap<String, AggregatedEntry>,
    /// Cache of package information by package name.
    cache: HashMap<String, PkgInfo>,
    /// Set of missing package names.
    missing: HashSet<String>,
    /// Set of target package names for reverse dependency resolution.
    target_names: HashSet<String>,
}

impl ReverseResolverState {
    /// What: Initialize traversal state for the provided removal targets.
    ///
    /// Inputs:
    /// - `targets`: Packages selected for removal.
    ///
    /// Output:
    /// - Returns a state object preloaded with target name bookkeeping.
    ///
    /// Details:
    /// - Prepares aggregation maps and caches so subsequent queries can avoid redundant pacman calls.
    fn new(targets: &[PackageRef]) -> Self {
        let target_names = targets.iter().map(|pkg| pkg.name.clone()).collect();
        Self {
            aggregated: HashMap::new(),
            cache: HashMap::new(),
            missing: HashSet::new(),
            target_names,
        }
    }

    /// What: Fetch and cache package information for a given name.
    ///
    /// Inputs:
    /// - `name`: Package whose metadata should be retrieved via `pacman -Qi`.
    ///
    /// Output:
    /// - Returns package info when available; otherwise caches the miss and yields `None`.
    ///
    /// Details:
    /// - Avoids repeated command executions by memoizing both hits and misses across the traversal.
    fn pkg_info(&mut self, name: &str) -> Option<PkgInfo> {
        if let Some(info) = self.cache.get(name) {
            return Some(info.clone());
        }
        if self.missing.contains(name) {
            return None;
        }

        match fetch_pkg_info(name) {
            Ok(info) => {
                self.cache.insert(name.to_string(), info.clone());
                Some(info)
            }
            Err(err) => {
                tracing::warn!("Failed to query pacman -Qi {}: {}", name, err);
                self.missing.insert(name.to_string());
                None
            }
        }
    }

    /// What: Update aggregation records to reflect a discovered reverse dependency relationship.
    ///
    /// Inputs:
    /// - `dependent`: Package that depends on the current node.
    /// - `parent`: Immediate package causing the dependency (may be empty).
    /// - `root`: Root removal target currently being explored.
    /// - `depth`: Distance from the root in the traversal.
    ///
    /// Output:
    /// - Mutates internal maps to capture per-root relationships and selection flags.
    ///
    /// Details:
    /// - Consolidates metadata per dependent package while preserving shortest depth and parent sets per root.
    fn update_entry(&mut self, dependent: &str, parent: &str, root: &str, depth: usize) {
        if dependent.eq_ignore_ascii_case(root) {
            return;
        }

        let Some(info) = self.pkg_info(dependent) else {
            return;
        };

        let selected = self.target_names.contains(dependent);
        match self.aggregated.entry(dependent.to_owned()) {
            Entry::Occupied(mut entry) => {
                let data = entry.get_mut();
                data.info = info;
                if selected {
                    data.selected_for_removal = true;
                }
                let relation = data
                    .per_root
                    .entry(root.to_string())
                    .or_insert_with(RootRelation::new);
                relation.record(parent, depth);
            }
            Entry::Vacant(slot) => {
                let mut data = AggregatedEntry {
                    info,
                    per_root: HashMap::new(),
                    selected_for_removal: selected,
                };
                data.per_root
                    .entry(root.to_string())
                    .or_insert_with(RootRelation::new)
                    .record(parent, depth);
                slot.insert(data);
            }
        }
    }
}

/// What: Snapshot of metadata retrieved from pacman's local database for traversal decisions.
///
/// Inputs:
/// - Filled by `fetch_pkg_info`, capturing fields relevant to reverse dependency aggregation.
///
/// Output:
/// - Provides reusable package details to avoid multiple CLI invocations.
///
/// Details:
/// - Stores only the subset of fields necessary for summarising conflicts and dependencies.
#[derive(Clone, Debug)]
struct PkgInfo {
    /// Package name.
    name: String,
    /// Package version.
    #[allow(dead_code)] // Version is fetched but not currently used in convert_entry
    version: String,
    /// Repository name (None for AUR packages).
    repo: Option<String>,
    /// Package groups.
    groups: Vec<String>,
    /// Packages that require this package.
    required_by: Vec<String>,
    /// Whether package was explicitly installed.
    explicit: bool,
}

/// What: Aggregated view of a dependent package across all removal roots.
///
/// Inputs:
/// - Populated incrementally as `update_entry` discovers new relationships.
///
/// Output:
/// - Captures per-root metadata along with selection status for downstream conversion.
///
/// Details:
/// - Maintains deduplicated parent sets for each root to explain conflict chains clearly.
#[derive(Clone, Debug)]
struct AggregatedEntry {
    /// Package information.
    info: PkgInfo,
    /// Relationship information per removal root.
    per_root: HashMap<String, RootRelation>,
    /// Whether this package is selected for removal.
    selected_for_removal: bool,
}

/// What: Relationship summary between a dependent package and a particular removal root.
///
/// Inputs:
/// - Updated as traversal discovers parents contributing to the dependency.
///
/// Output:
/// - Tracks unique parent names and the minimum depth from the root.
///
/// Details:
/// - Used to distinguish direct versus transitive dependents in the final summary.
#[derive(Clone, Debug)]
struct RootRelation {
    /// Set of parent package names that contribute to this dependency.
    parents: HashSet<String>,
    /// Minimum depth from the removal root to this package.
    min_depth: usize,
}

impl RootRelation {
    /// What: Construct an empty relation ready to collect parent metadata.
    ///
    /// Inputs:
    /// - (none): Starts with default depth and empty parent set.
    ///
    /// Output:
    /// - Returns a relation with `usize::MAX` depth and no parents recorded.
    ///
    /// Details:
    /// - The sentinel depth ensures first updates always win when computing minimum distance.
    fn new() -> Self {
        Self {
            parents: HashSet::new(),
            min_depth: usize::MAX,
        }
    }

    /// What: Record a traversal parent contributing to the dependency chain.
    ///
    /// Inputs:
    /// - `parent`: Name of the package one level closer to the root.
    /// - `depth`: Current depth from the root target.
    ///
    /// Output:
    /// - Updates internal parent set and minimum depth as appropriate.
    ///
    /// Details:
    /// - Ignores empty parent identifiers and keeps the shallowest depth observed for summarisation.
    fn record(&mut self, parent: &str, depth: usize) {
        if !parent.is_empty() {
            self.parents.insert(parent.to_string());
        }
        if depth < self.min_depth {
            self.min_depth = depth;
        }
    }

    /// What: Report the closest distance from this dependent to the root target.
    ///
    /// Inputs:
    /// - (none): Uses previously recorded depth values.
    ///
    /// Output:
    /// - Returns the smallest depth stored during traversal.
    ///
    /// Details:
    /// - Allows callers to classify dependencies as direct when the minimum depth is one.
    const fn min_depth(&self) -> usize {
        self.min_depth
    }
}

/// What: Convert an aggregated reverse dependency entry into UI-facing metadata.
///
/// Inputs:
/// - `name`: Canonical dependent package name.
/// - `entry`: Aggregated structure containing metadata and per-root relations.
///
/// Output:
/// - Returns a `Dependency` tailored for preflight summaries with conflict reasoning.
///
/// Details:
/// - Merges parent sets, sorts presentation fields, and infers system/core flags for display.
fn convert_entry(name: String, entry: AggregatedEntry) -> Dependency {
    let AggregatedEntry {
        info,
        per_root,
        selected_for_removal,
    } = entry;

    let PkgInfo {
        name: pkg_name,
        version: _,
        repo,
        groups,
        required_by: _,
        explicit,
    } = info;

    let mut required_by: Vec<String> = per_root.keys().cloned().collect();
    required_by.sort();

    let mut all_parents: HashSet<String> = HashSet::new();
    for relation in per_root.values() {
        all_parents.extend(relation.parents.iter().cloned());
    }
    let mut depends_on: Vec<String> = all_parents.into_iter().collect();
    depends_on.sort();

    let mut reason_parts: Vec<String> = Vec::new();
    for (root, relation) in &per_root {
        let depth = relation.min_depth();
        let mut parents: Vec<String> = relation.parents.iter().cloned().collect();
        parents.sort();

        if depth <= 1 {
            reason_parts.push(format!("requires {root}"));
        } else {
            let via = if parents.is_empty() {
                "unknown".to_string()
            } else {
                parents.join(", ")
            };
            reason_parts.push(format!("blocks {root} (depth {depth} via {via})"));
        }
    }

    if selected_for_removal {
        reason_parts.push("already selected for removal".to_string());
    }
    if explicit {
        reason_parts.push("explicitly installed".to_string());
    }

    reason_parts.sort();
    let reason = if reason_parts.is_empty() {
        "required by removal targets".to_string()
    } else {
        reason_parts.join("; ")
    };

    let source = match repo.as_deref() {
        Some(repo) if repo.eq_ignore_ascii_case("local") || repo.is_empty() => {
            DependencySource::Local
        }
        Some(repo) => DependencySource::Official {
            repo: repo.to_string(),
        },
        None => DependencySource::Local,
    };

    let is_core = repo
        .as_deref()
        .is_some_and(|r| r.eq_ignore_ascii_case("core"));
    let is_system = groups
        .iter()
        .any(|g| matches!(g.as_str(), "base" | "base-devel"));

    let display_name = if pkg_name.is_empty() { name } else { pkg_name };

    Dependency {
        name: display_name,
        version_req: String::new(), // Reverse deps don't have version requirements
        status: DependencyStatus::Conflict { reason },
        source,
        required_by,
        depends_on,
        is_core,
        is_system,
    }
}

/// What: Query pacman for detailed information about an installed package.
///
/// Inputs:
/// - `name`: Package name passed to `pacman -Qi`.
///
/// Output:
/// - Returns a `PkgInfo` snapshot or an `ArchToolkitError` if the query fails.
///
/// Details:
/// - Parses key-value fields such as repository, groups, and required-by lists for downstream processing.
/// - Sets `LC_ALL=C` and `LANG=C` for consistent locale-independent output.
fn fetch_pkg_info(name: &str) -> Result<PkgInfo> {
    tracing::debug!("Running: pacman -Qi {}", name);
    let output = Command::new("pacman")
        .args(["-Qi", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ArchToolkitError::Parse(format!("pacman -Qi {name} failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ArchToolkitError::Parse(format!(
            "pacman -Qi {name} exited with {:?}: {}",
            output.status, stderr
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let map = parse_key_value_output(&text);

    let required_by = split_ws_or_none(map.get("Required By"));
    let groups = split_ws_or_none(map.get("Groups"));
    let version = map.get("Version").cloned().unwrap_or_default();
    let repo = map.get("Repository").cloned();
    let install_reason = map
        .get("Install Reason")
        .cloned()
        .unwrap_or_default()
        .to_lowercase();
    let explicit = install_reason.contains("explicit");

    Ok(PkgInfo {
        name: map.get("Name").cloned().unwrap_or_else(|| name.to_string()),
        version,
        repo,
        groups,
        required_by,
        explicit,
    })
}

/// What: Parse pacman key-value output into a searchable map.
///
/// Inputs:
/// - `text`: Multi-line output containing colon-separated fields with optional wrapped lines.
///
/// Output:
/// - Returns a `BTreeMap` mapping field names to their consolidated string values.
///
/// Details:
/// - Handles indented continuation lines by appending them to the most recently parsed key.
fn parse_key_value_output(text: &str) -> BTreeMap<String, String> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut last_key: Option<String> = None;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_string();
            let val = v.trim().to_string();
            last_key = Some(key.clone());
            map.insert(key, val);
        } else if (line.starts_with(' ') || line.starts_with('\t'))
            && let Some(key) = &last_key
        {
            let entry = map.entry(key.clone()).or_default();
            if !entry.ends_with(' ') {
                entry.push(' ');
            }
            entry.push_str(line.trim());
        }
    }

    map
}

/// What: Break a whitespace-separated field into individual tokens, ignoring sentinel values.
///
/// Inputs:
/// - `field`: Optional string obtained from pacman metadata.
///
/// Output:
/// - Returns a vector of tokens or an empty vector when the field is missing or marked as "None".
///
/// Details:
/// - Trims surrounding whitespace before evaluating the contents to avoid spurious blank entries.
fn split_ws_or_none(field: Option<&String>) -> Vec<String> {
    field.map_or_else(Vec::new, |value| {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            Vec::new()
        } else {
            trimmed
                .split_whitespace()
                .map(ToString::to_string)
                .collect()
        }
    })
}

/// What: Check if a package has any installed packages in its "Required By" field.
///
/// Inputs:
/// - `name`: Package name to check.
///
/// Output:
/// - Returns `true` if the package has at least one installed package in its "Required By" field, `false` otherwise.
///
/// Details:
/// - Runs `pacman -Qi` to query package information and parses the "Required By" field.
/// - Checks each package in "Required By" against the installed package cache.
/// - Returns `false` if the package is not installed or if querying fails.
/// - Gracefully degrades when pacman is unavailable (returns `false`).
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::has_installed_required_by;
///
/// if has_installed_required_by("glibc") {
///     println!("glibc has installed dependents");
/// }
/// ```
#[must_use]
pub fn has_installed_required_by(name: &str) -> bool {
    let Ok(installed) = get_installed_packages() else {
        tracing::debug!("Failed to get installed packages for has_installed_required_by");
        return false;
    };

    match fetch_pkg_info(name) {
        Ok(info) => info
            .required_by
            .iter()
            .any(|pkg| installed.contains(pkg.as_str())),
        Err(err) => {
            tracing::debug!("Failed to query pacman -Qi {}: {}", name, err);
            false
        }
    }
}

/// What: Get the list of installed packages that depend on a package.
///
/// Inputs:
/// - `name`: Package name to check.
///
/// Output:
/// - Returns a vector of package names that are installed and depend on the package, or an empty vector on failure.
///
/// Details:
/// - Runs `pacman -Qi` to query package information and parses the "Required By" field.
/// - Filters the "Required By" list to only include installed packages.
/// - Returns an empty vector if the package is not installed or if querying fails.
/// - Gracefully degrades when pacman is unavailable (returns empty vector).
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::get_installed_required_by;
///
/// let dependents = get_installed_required_by("glibc");
/// println!("Found {} installed dependents", dependents.len());
/// ```
#[must_use]
pub fn get_installed_required_by(name: &str) -> Vec<String> {
    let Ok(installed) = get_installed_packages() else {
        tracing::debug!("Failed to get installed packages for get_installed_required_by");
        return Vec::new();
    };

    match fetch_pkg_info(name) {
        Ok(info) => info
            .required_by
            .iter()
            .filter(|pkg| installed.contains(pkg.as_str()))
            .cloned()
            .collect(),
        Err(err) => {
            tracing::debug!("Failed to query pacman -Qi {}: {}", name, err);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::dependency::PackageSource;

    fn pkg_ref(name: &str) -> PackageRef {
        PackageRef {
            name: name.into(),
            version: "1.0".into(),
            source: PackageSource::Official {
                repo: "extra".into(),
                arch: "x86_64".into(),
            },
        }
    }

    fn pkg_info_stub(name: &str) -> PkgInfo {
        PkgInfo {
            name: name.into(),
            version: "2.0".into(),
            repo: Some("extra".into()),
            groups: Vec::new(),
            required_by: Vec::new(),
            explicit: false,
        }
    }

    #[test]
    /// What: Verify `update_entry` marks target packages and records per-root relations correctly.
    ///
    /// Inputs:
    /// - `targets`: Root and dependent package items forming the resolver seed.
    /// - `state`: Fresh `ReverseResolverState` with cached info for the dependent package.
    ///
    /// Output:
    /// - Aggregated entry reflects selection, contains relation for the root, and tracks parents.
    ///
    /// Details:
    /// - Ensures depth calculation and parent recording occur when updating the entry for a target
    ///   package linked to a specified root.
    fn update_entry_tracks_root_relations_and_selection() {
        let targets = vec![pkg_ref("root"), pkg_ref("app")];
        let mut state = ReverseResolverState::new(&targets);
        state.cache.insert("app".into(), pkg_info_stub("app"));

        state.update_entry("app", "root", "root", 1);

        let entry = state
            .aggregated
            .get("app")
            .expect("aggregated entry populated");
        assert!(entry.selected_for_removal, "target membership flagged");
        assert_eq!(entry.info.name, "app");
        let relation = entry
            .per_root
            .get("root")
            .expect("relation stored for root");
        assert_eq!(relation.min_depth(), 1);
        assert!(relation.parents.contains("root"));
    }

    #[test]
    /// What: Confirm `convert_entry` surfaces conflict reasons, metadata, and flags accurately.
    ///
    /// Inputs:
    /// - `entry`: Aggregated dependency entry with multiple root relations and metadata toggles.
    ///
    /// Output:
    /// - Resulting `Dependency` carries conflict status, sorted relations, and flag booleans.
    ///
    /// Details:
    /// - Validates that reasons mention blocking roots, selection state, explicit install, and core/system
    ///   classification while preserving alias names and parent ordering.
    fn convert_entry_produces_conflict_reason_and_flags() {
        let mut relation_a = RootRelation::new();
        relation_a.record("root", 1);
        let mut relation_b = RootRelation::new();
        relation_b.record("parent_x", 2);
        relation_b.record("parent_y", 2);

        let entry = AggregatedEntry {
            info: PkgInfo {
                name: "dep_alias".into(),
                version: "3.1".into(),
                repo: Some("core".into()),
                groups: vec!["base".into()],
                required_by: Vec::new(),
                explicit: true,
            },
            per_root: HashMap::from([("root".into(), relation_a), ("other".into(), relation_b)]),
            selected_for_removal: true,
        };

        let info = convert_entry("dep".into(), entry);
        let DependencyStatus::Conflict { reason } = &info.status else {
            panic!("expected conflict status");
        };
        assert!(reason.contains("requires root"));
        assert!(reason.contains("blocks other"));
        assert!(reason.contains("already selected for removal"));
        assert!(reason.contains("explicitly installed"));
        assert_eq!(info.required_by, vec!["other", "root"]);
        assert_eq!(info.depends_on, vec!["parent_x", "parent_y", "root"]);
        assert!(info.is_core);
        assert!(info.is_system);
        assert_eq!(info.name, "dep_alias");
    }

    #[test]
    /// What: Ensure pacman-style key/value parsing merges wrapped descriptions.
    ///
    /// Inputs:
    /// - `sample`: Multi-line text where description continues on the next indented line.
    ///
    /// Output:
    /// - Parsed map flattens wrapped lines and retains other keys verbatim.
    ///
    /// Details:
    /// - Simulates `pacman -Qi` output to verify `parse_key_value_output` concatenates continuation
    ///   lines into a single value.
    fn parse_key_value_output_merges_wrapped_lines() {
        let sample = "Name            : pkg\nDescription     : Short desc\n                continuation line\nRequired By     : foo bar\nInstall Reason  : Explicitly installed\n";
        let map = parse_key_value_output(sample);
        assert_eq!(map.get("Name"), Some(&"pkg".to_string()));
        assert_eq!(
            map.get("Description"),
            Some(&"Short desc continuation line".to_string())
        );
        assert_eq!(map.get("Required By"), Some(&"foo bar".to_string()));
    }

    #[test]
    /// What: Validate whitespace splitting helper ignores empty and "none" values.
    ///
    /// Inputs:
    /// - `field`: Optional strings containing "None", whitespace, words, or `None`.
    ///
    /// Output:
    /// - Returns empty vector for none-like inputs and splits valid whitespace-separated tokens.
    ///
    /// Details:
    /// - Covers uppercase "None", blank strings, regular word lists, and the absence of a value.
    fn split_ws_or_none_handles_none_and_empty() {
        assert!(split_ws_or_none(Some(&"None".to_string())).is_empty());
        assert!(split_ws_or_none(Some(&"   ".to_string())).is_empty());
        let list = split_ws_or_none(Some(&"foo bar".to_string()));
        assert_eq!(list, vec!["foo", "bar"]);
        assert!(split_ws_or_none(None).is_empty());
    }

    #[test]
    /// What: Test `RootRelation` depth tracking and parent recording.
    ///
    /// Inputs:
    /// - `relation`: Fresh `RootRelation` instance.
    ///
    /// Output:
    /// - Relation correctly tracks minimum depth and parent sets.
    ///
    /// Details:
    /// - Verifies that depth is updated to minimum value and parents are accumulated.
    fn root_relation_tracks_depth_and_parents() {
        let mut relation = RootRelation::new();
        assert_eq!(relation.min_depth(), usize::MAX);

        relation.record("parent1", 2);
        assert_eq!(relation.min_depth(), 2);
        assert!(relation.parents.contains("parent1"));

        relation.record("parent2", 1);
        assert_eq!(relation.min_depth(), 1);
        assert!(relation.parents.contains("parent1"));
        assert!(relation.parents.contains("parent2"));

        relation.record("", 3); // Empty parent should be ignored
        assert_eq!(relation.min_depth(), 1);
        assert_eq!(relation.parents.len(), 2);
    }
}

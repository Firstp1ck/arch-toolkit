//! Bounded, fixture-friendly dependency graph resolution.
//!
//! This module keeps graph expansion separate from the legacy synchronous host resolver so callers
//! can inject verified `.SRCINFO` metadata without enabling the `aur` feature.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use crate::deps::parse::parse_dep_spec;
use crate::deps::srcinfo::{GraphSrcinfoData, SrcinfoPackage, parse_srcinfo_graph};
use crate::deps::version::{compare_versions, version_satisfies};
use crate::error::{ArchToolkitError, Result};
use crate::types::dependency::{
    DependencyConstraintRange, DependencyGraphConfig, DependencyGraphDiagnostic,
    DependencyGraphDiagnosticKind, DependencyGraphEdge, DependencyGraphNode,
    DependencyGraphNodeStatus, DependencyGraphResolution, DependencyMetadata,
    DependencyMetadataResponse, DependencyProvenance, DependencyVersionBound, PackageRef,
};

/// What: Fetch verified raw `.SRCINFO` metadata for graph-resolution requests.
///
/// Inputs:
/// - A lexically sorted batch of requested package or virtual names.
/// - A per-batch timeout supplied by `DependencyGraphConfig`.
///
/// Output:
/// - Returns one `DependencyMetadataResponse` per request whenever possible.
///
/// Details:
/// - The trait belongs to `deps` and has no AUR, HTTP, runtime, or helper dependency. Providers
///   may batch internally and must honor the supplied timeout for cancellable I/O. The synchronous
///   resolver issues batches serially, so it never has more than one provider call in flight.
pub trait DependencyMetadataProvider: Send + Sync {
    /// What: Retrieve raw metadata for a bounded batch of requested names.
    ///
    /// Inputs:
    /// - `requested_names`: Lexically sorted, unique requested package or virtual names.
    /// - `timeout`: Maximum time allocated to this provider batch.
    ///
    /// Output:
    /// - Returns found, missing, or failed metadata responses.
    ///
    /// Details:
    /// - Providers should return a response for every input. The resolver records a structured
    ///   protocol diagnostic for omitted, duplicate, or unrequested responses.
    fn fetch_metadata(
        &self,
        requested_names: &[String],
        timeout: Duration,
    ) -> Vec<DependencyMetadataResponse>;
}

/// What: Track one dependency request waiting for metadata processing.
///
/// Inputs:
/// - Parent package, requested name, version requirement, depth, and active path.
///
/// Output:
/// - Carries deterministic traversal state between bounded provider batches.
///
/// Details:
/// - `path` contains actual selected package names, permitting cycle detection after virtual
///   provider selection.
#[derive(Clone, Debug)]
struct PendingRequest {
    /// Actual parent package, or `None` for a root request.
    parent: Option<String>,
    /// Requested package or virtual dependency name.
    requested_name: String,
    /// Requested version requirement.
    version_req: String,
    /// Edge depth from a root.
    depth: usize,
    /// Actual package names active on the traversal path.
    path: Vec<String>,
}

/// What: Validate graph-resolution bounds before metadata lookup.
///
/// Inputs:
/// - `config`: Caller-provided graph bounds.
///
/// Output:
/// - Returns `Ok(())` for usable bounds or an actionable invalid-input error.
///
/// Details:
/// - Zero node, timeout, or provider-batch limits are rejected rather than silently disabling a
///   safety bound. A zero depth remains valid and resolves root metadata only.
fn validate_graph_config(config: &DependencyGraphConfig) -> Result<()> {
    if config.max_nodes == 0 {
        return Err(ArchToolkitError::InvalidInput(
            "dependency graph max_nodes must be greater than zero".to_string(),
        ));
    }
    if config.metadata_timeout.is_zero() {
        return Err(ArchToolkitError::InvalidInput(
            "dependency graph metadata_timeout must be greater than zero".to_string(),
        ));
    }
    if config.max_concurrency == 0 {
        return Err(ArchToolkitError::InvalidInput(
            "dependency graph max_concurrency must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

/// What: Sort pending requests into deterministic breadth-first lexical order.
///
/// Inputs:
/// - `pending`: Requests waiting for cache lookup or metadata processing.
///
/// Output:
/// - Updates the vector in depth, requested-name, parent, and constraint order.
///
/// Details:
/// - Stable ordering ensures provider batch order, diagnostics, graph edges, and rendering do not
///   depend on caller root order or provider response order.
fn sort_pending(pending: &mut [PendingRequest]) {
    pending.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.requested_name.cmp(&right.requested_name))
            .then_with(|| left.parent.cmp(&right.parent))
            .then_with(|| left.version_req.cmp(&right.version_req))
    });
}

/// What: Return the requested name carried by a provider response.
///
/// Inputs:
/// - `response`: One metadata provider response.
///
/// Output:
/// - Returns the response's requested package or virtual name.
///
/// Details:
/// - This allows the resolver to reject batch protocol mismatches before parsing metadata.
fn response_requested_name(response: &DependencyMetadataResponse) -> &str {
    match response {
        DependencyMetadataResponse::Found(metadata) => &metadata.requested_name,
        DependencyMetadataResponse::Missing { requested_name, .. }
        | DependencyMetadataResponse::Failure { requested_name, .. } => requested_name,
    }
}

/// What: Add a structured graph diagnostic.
///
/// Inputs:
/// - `diagnostics`: Destination diagnostics.
/// - `kind`: Stable event category.
/// - `package`: Affected package or requested name.
/// - `related_package`: Optional related package.
/// - `message`: Actionable event detail.
///
/// Output:
/// - Appends one diagnostic entry.
///
/// Details:
/// - Final result sorting makes diagnostic order deterministic regardless of provider ordering.
fn push_diagnostic(
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
    kind: DependencyGraphDiagnosticKind,
    package: impl Into<String>,
    related_package: Option<String>,
    message: impl Into<String>,
) {
    diagnostics.push(DependencyGraphDiagnostic {
        kind,
        package: package.into(),
        related_package,
        message: message.into(),
    });
}

/// What: Cache one bounded provider batch and diagnose protocol violations.
///
/// Inputs:
/// - `requests`: Unique requested names sent to the provider.
/// - `responses`: Provider output for the batch.
/// - `cache`: Per-run metadata response cache.
/// - `diagnostics`: Resolution diagnostics.
///
/// Output:
/// - Caches a response or deterministic synthetic failure for every request.
///
/// Details:
/// - Duplicate and unrequested responses are diagnosed. Missing responses become failures so the
///   resolver does not silently issue repeat requests or infer package provenance.
fn cache_batch_responses(
    requests: &[String],
    responses: Vec<DependencyMetadataResponse>,
    cache: &mut BTreeMap<String, DependencyMetadataResponse>,
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
) {
    let requested = requests.iter().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for response in responses {
        let response_name = response_requested_name(&response).to_string();
        if !requested.contains(&response_name) {
            push_diagnostic(
                diagnostics,
                DependencyGraphDiagnosticKind::MetadataProtocol,
                response_name,
                None,
                "metadata provider returned a response for an unrequested name",
            );
            continue;
        }
        if !seen.insert(response_name.clone()) {
            push_diagnostic(
                diagnostics,
                DependencyGraphDiagnosticKind::MetadataProtocol,
                response_name,
                None,
                "metadata provider returned duplicate responses for one request",
            );
            continue;
        }
        cache.insert(response_name, response);
    }
    for request in requests {
        if !seen.contains(request) {
            push_diagnostic(
                diagnostics,
                DependencyGraphDiagnosticKind::MetadataProtocol,
                request,
                None,
                "metadata provider omitted a response for the request",
            );
            cache.insert(
                request.clone(),
                DependencyMetadataResponse::Failure {
                    requested_name: request.clone(),
                    message: "metadata provider omitted a response".to_string(),
                },
            );
        }
    }
}

/// What: Insert a graph node without exceeding the configured per-run node bound.
///
/// Inputs:
/// - `node`: Candidate graph node.
/// - `nodes`: Nodes indexed by actual package name.
/// - `config`: Graph-resolution bounds.
/// - `diagnostics`: Resolution diagnostics.
///
/// Output:
/// - Returns `true` when the node exists after the call and `false` when the node limit blocks it.
///
/// Details:
/// - Existing nodes are always reusable. New-node rejection is explicit and leaves the branch
///   unexpanded rather than exceeding the configured resource bound.
fn insert_node_if_allowed(
    node: DependencyGraphNode,
    nodes: &mut BTreeMap<String, DependencyGraphNode>,
    config: &DependencyGraphConfig,
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
) -> bool {
    if nodes.contains_key(&node.name) {
        return true;
    }
    if nodes.len() >= config.max_nodes {
        push_diagnostic(
            diagnostics,
            DependencyGraphDiagnosticKind::NodeLimit,
            &node.name,
            None,
            format!("dependency graph node limit ({}) reached", config.max_nodes),
        );
        return false;
    }
    nodes.insert(node.name.clone(), node);
    true
}

/// What: Build a missing graph node without inventing source provenance.
///
/// Inputs:
/// - `name`: Requested package or virtual name.
/// - `depth`: Traversal depth where metadata became unavailable.
/// - `source`: Verified source if malformed metadata existed, otherwise `None`.
///
/// Output:
/// - Returns a graph node in `Missing` state.
///
/// Details:
/// - Missing nodes provide an edge target for partial graph consumers while retaining the policy
///   that an unknown package is not automatically an AUR package.
fn missing_node(
    name: &str,
    depth: usize,
    source: Option<crate::types::dependency::DependencySource>,
) -> DependencyGraphNode {
    DependencyGraphNode {
        name: name.to_string(),
        pkgbase: None,
        version: None,
        provenance: DependencyProvenance {
            requested_name: name.to_string(),
            source,
            provider: None,
        },
        status: DependencyGraphNodeStatus::Missing,
        constraints: DependencyConstraintRange::default(),
        provides: Vec::new(),
        conflicts: Vec::new(),
        depth,
    }
}

/// What: Construct a package version retaining epoch and pkgrel semantics.
///
/// Inputs:
/// - `data`: Parsed `.SRCINFO` metadata for the selected package base.
///
/// Output:
/// - Returns an optional `epoch:pkgver-pkgrel` version string.
///
/// Details:
/// - Empty package version metadata is left absent so constraints are not claimed to be verified.
fn srcinfo_version(data: &GraphSrcinfoData) -> Option<String> {
    if data.pkgver.is_empty() {
        return None;
    }
    let epoch_prefix = if data.epoch.is_empty() {
        String::new()
    } else {
        format!("{}:", data.epoch)
    };
    let pkgrel_suffix = if data.pkgrel.is_empty() {
        String::new()
    } else {
        format!("-{}", data.pkgrel)
    };
    Some(format!("{epoch_prefix}{}{pkgrel_suffix}", data.pkgver))
}

/// What: Check that an actual provider output verifies a virtual requested name.
///
/// Inputs:
/// - `package`: Selected package output from `.SRCINFO`.
/// - `requested_name`: Original virtual dependency name.
/// - `version_req`: Requested virtual version constraint.
///
/// Output:
/// - Returns `true` when a matching `provides` entry verifies the request.
///
/// Details:
/// - An unversioned provide satisfies only an unversioned request. A versioned provide is compared
///   using the same epoch/pkgver/pkgrel comparator as ordinary dependency constraints.
fn provider_satisfies_request(
    package: &SrcinfoPackage,
    requested_name: &str,
    version_req: &str,
) -> bool {
    package.provides.iter().any(|provided| {
        let provided_spec = parse_dep_spec(provided);
        if provided_spec.name != requested_name {
            return false;
        }
        if version_req.is_empty() {
            return true;
        }
        let Some(version) = provided_spec.version_req.strip_prefix('=') else {
            return false;
        };
        version_satisfies(version, version_req)
    })
}

/// What: Parse and intersect one dependency requirement with a version range.
///
/// Inputs:
/// - `range`: Existing compatible range.
/// - `requirement`: A dependency operator and version, or an empty requirement.
///
/// Output:
/// - Returns an intersected range or `None` when the requirement is malformed or incompatible.
///
/// Details:
/// - The interval supports `=`, `>`, `>=`, `<`, and `<=`; exact constraints become equal inclusive
///   lower and upper bounds. The function never inspects host package state.
fn intersect_requirement(
    range: &DependencyConstraintRange,
    requirement: &str,
) -> Option<DependencyConstraintRange> {
    if requirement.is_empty() {
        return Some(range.clone());
    }
    let (operator, version) = [">=", "<=", "=", ">", "<"].iter().find_map(|operator| {
        requirement
            .strip_prefix(operator)
            .map(|version| (*operator, version))
    })?;
    if version.is_empty() {
        return None;
    }
    let mut candidate = range.clone();
    let bound = DependencyVersionBound {
        version: version.to_string(),
        inclusive: matches!(operator, ">=" | "<=" | "="),
    };
    match operator {
        ">" | ">=" => update_lower(&mut candidate.lower, bound),
        "<" | "<=" => update_upper(&mut candidate.upper, bound),
        "=" => {
            update_lower(&mut candidate.lower, bound.clone());
            update_upper(&mut candidate.upper, bound);
        }
        _ => return None,
    }
    range_is_valid(&candidate).then_some(candidate)
}

/// What: Update an interval lower bound with the more restrictive requirement.
///
/// Inputs:
/// - `current`: Existing lower bound.
/// - `candidate`: New lower bound.
///
/// Output:
/// - Keeps the larger version, with exclusive equality taking precedence.
///
/// Details:
/// - This is pure constraint algebra and deliberately does not query installed packages.
fn update_lower(current: &mut Option<DependencyVersionBound>, candidate: DependencyVersionBound) {
    let should_replace = current.as_ref().is_none_or(|existing| {
        matches!(
            compare_versions(&candidate.version, &existing.version),
            std::cmp::Ordering::Greater
        ) || (candidate.version == existing.version && !candidate.inclusive && existing.inclusive)
    });
    if should_replace {
        *current = Some(candidate);
    }
}

/// What: Update an interval upper bound with the more restrictive requirement.
///
/// Inputs:
/// - `current`: Existing upper bound.
/// - `candidate`: New upper bound.
///
/// Output:
/// - Keeps the smaller version, with exclusive equality taking precedence.
///
/// Details:
/// - This is pure constraint algebra and deliberately does not query installed packages.
fn update_upper(current: &mut Option<DependencyVersionBound>, candidate: DependencyVersionBound) {
    let should_replace = current.as_ref().is_none_or(|existing| {
        matches!(
            compare_versions(&candidate.version, &existing.version),
            std::cmp::Ordering::Less
        ) || (candidate.version == existing.version && !candidate.inclusive && existing.inclusive)
    });
    if should_replace {
        *current = Some(candidate);
    }
}

/// What: Determine whether an intersected version interval contains any version.
///
/// Inputs:
/// - `range`: Candidate lower and upper version bounds.
///
/// Output:
/// - Returns `true` for compatible or unbounded ranges.
///
/// Details:
/// - Equal bounds are incompatible whenever either side excludes equality.
fn range_is_valid(range: &DependencyConstraintRange) -> bool {
    let (Some(lower), Some(upper)) = (&range.lower, &range.upper) else {
        return true;
    };
    match compare_versions(&lower.version, &upper.version) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => lower.inclusive && upper.inclusive,
    }
}

/// What: Merge an edge constraint into a selected graph node.
///
/// Inputs:
/// - `node`: Selected graph node.
/// - `requirement`: Direct-package version requirement from an incoming edge.
/// - `parent`: Parent package that declared the requirement.
/// - `diagnostics`: Resolution diagnostics.
///
/// Output:
/// - Updates the node's compatible range or emits an incompatibility diagnostic.
///
/// Details:
/// - Virtual-provider constraints are validated against `provides` separately and are not applied to
///   the provider's own package version interval.
fn merge_node_requirement(
    node: &mut DependencyGraphNode,
    requirement: &str,
    parent: Option<&str>,
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
) {
    if !requirement_is_well_formed(requirement) {
        push_diagnostic(
            diagnostics,
            DependencyGraphDiagnosticKind::MalformedConstraint,
            &node.name,
            parent.map(str::to_string),
            format!("requirement '{requirement}' has no supported operator and version"),
        );
        return;
    }
    let Some(range) = intersect_requirement(&node.constraints, requirement) else {
        push_diagnostic(
            diagnostics,
            DependencyGraphDiagnosticKind::IncompatibleConstraints,
            &node.name,
            parent.map(str::to_string),
            format!("requirement '{requirement}' has no compatible intersection"),
        );
        return;
    };
    node.constraints = range;
}

/// What: Validate the syntax accepted by graph constraint intersection.
///
/// Inputs:
/// - `requirement`: Empty requirement or an operator-prefixed version requirement.
///
/// Output:
/// - `true` for empty requirements and supported operators with a non-empty version.
///
/// Details:
/// - Keeps malformed metadata diagnostics distinct from valid but incompatible intervals.
fn requirement_is_well_formed(requirement: &str) -> bool {
    requirement.is_empty()
        || [">=", "<=", "=", ">", "<"]
            .iter()
            .find_map(|operator| requirement.strip_prefix(operator))
            .is_some_and(|version| !version.is_empty())
}

/// What: Add a graph edge while preserving a stable duplicate-free result.
///
/// Inputs:
/// - `edges`: Existing graph edges.
/// - `parent`: Actual parent package.
/// - `child`: Actual selected or missing child package.
/// - `requested_name`: Dependency name written by the parent.
/// - `version_req`: Requested version constraint.
///
/// Output:
/// - Adds the edge only when an identical edge is not already present.
///
/// Details:
/// - Multiple requirements for one package remain distinct edges when their constraints differ.
fn add_edge(
    edges: &mut Vec<DependencyGraphEdge>,
    parent: Option<&str>,
    child: &str,
    requested_name: &str,
    version_req: &str,
) {
    let Some(parent) = parent else {
        return;
    };
    let edge = DependencyGraphEdge {
        from: parent.to_string(),
        to: child.to_string(),
        requested_name: requested_name.to_string(),
        version_req: version_req.to_string(),
    };
    if !edges.contains(&edge) {
        edges.push(edge);
    }
}

/// What: Process an unavailable or failed metadata response.
///
/// Inputs:
/// - `pending`: Request being resolved.
/// - `response`: Missing or failed provider response.
/// - `nodes`, `edges`, `roots`, `config`, and `diagnostics`: Mutable graph state.
///
/// Output:
/// - Adds a partial missing node and edge when the node bound permits it.
///
/// Details:
/// - Provider errors remain structured diagnostics and do not abort unrelated resolution branches.
fn process_unavailable_metadata(
    pending: &PendingRequest,
    response: &DependencyMetadataResponse,
    nodes: &mut BTreeMap<String, DependencyGraphNode>,
    edges: &mut Vec<DependencyGraphEdge>,
    roots: &mut BTreeSet<String>,
    config: &DependencyGraphConfig,
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
) {
    let (kind, message) = match response {
        DependencyMetadataResponse::Missing { reason, .. } => (
            DependencyGraphDiagnosticKind::MissingMetadata,
            format!("metadata unavailable: {reason}"),
        ),
        DependencyMetadataResponse::Failure { message, .. } => (
            DependencyGraphDiagnosticKind::MetadataFailure,
            format!("metadata retrieval failed: {message}"),
        ),
        DependencyMetadataResponse::Found(_) => return,
    };
    push_diagnostic(
        diagnostics,
        kind,
        &pending.requested_name,
        pending.parent.clone(),
        message,
    );
    let node = missing_node(&pending.requested_name, pending.depth, None);
    if insert_node_if_allowed(node, nodes, config, diagnostics) {
        add_edge(
            edges,
            pending.parent.as_deref(),
            &pending.requested_name,
            &pending.requested_name,
            &pending.version_req,
        );
        if pending.parent.is_none() {
            roots.insert(pending.requested_name.clone());
        }
    }
}

/// What: Select and validate a package output from injected `.SRCINFO` metadata.
///
/// Inputs:
/// - `metadata`: Provider-returned raw `.SRCINFO` metadata.
/// - `pending`: Dependency request that selected the metadata.
/// - `diagnostics`: Resolution diagnostics.
///
/// Output:
/// - Returns the parsed data and selected package output when validation succeeds.
///
/// Details:
/// - Split packages are selected strictly by `metadata.package_name`. Virtual provider selections
///   must prove the originally requested name through a matching `provides` entry.
fn select_srcinfo_package(
    metadata: &DependencyMetadata,
    pending: &PendingRequest,
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
) -> Option<(GraphSrcinfoData, String)> {
    let data = parse_srcinfo_graph(&metadata.srcinfo);
    if !data
        .packages
        .iter()
        .any(|package| package.name == metadata.package_name)
    {
        push_diagnostic(
            diagnostics,
            DependencyGraphDiagnosticKind::MalformedSrcinfo,
            &pending.requested_name,
            pending.parent.clone(),
            format!(
                ".SRCINFO does not contain selected package output '{}'",
                metadata.package_name
            ),
        );
        return None;
    }
    let provider_verified = data
        .packages
        .iter()
        .find(|package| package.name == metadata.package_name)
        .is_some_and(|package| {
            provider_satisfies_request(package, &pending.requested_name, &pending.version_req)
        });
    if metadata.package_name != pending.requested_name && !provider_verified {
        push_diagnostic(
            diagnostics,
            DependencyGraphDiagnosticKind::MetadataProtocol,
            &pending.requested_name,
            pending.parent.clone(),
            format!(
                "selected provider '{}' does not verify requested virtual dependency",
                metadata.package_name
            ),
        );
        return None;
    }
    Some((data, metadata.package_name.clone()))
}

/// What: Collect included dependency fields from a selected `.SRCINFO` package.
///
/// Inputs:
/// - `package`: Selected package-output metadata.
/// - Inclusion flags from the legacy resolver configuration.
///
/// Output:
/// - Returns deduplicated dependency specifications in lexical order.
///
/// Details:
/// - Runtime dependencies are always included; optional, make, and check dependencies remain
///   opt-in to preserve the old resolver's configuration semantics.
fn selected_dependencies(
    package: &SrcinfoPackage,
    include_optdepends: bool,
    include_makedepends: bool,
    include_checkdepends: bool,
) -> Vec<String> {
    let mut dependencies = package.depends.clone();
    if include_optdepends {
        dependencies.extend(package.optdepends.iter().map(|dependency| {
            dependency.split_once(':').map_or_else(
                || dependency.clone(),
                |(_, target)| target.trim().to_string(),
            )
        }));
    }
    if include_makedepends {
        dependencies.extend(package.makedepends.clone());
    }
    if include_checkdepends {
        dependencies.extend(package.checkdepends.clone());
    }
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

/// What: Process verified metadata and enqueue its bounded child dependency requests.
///
/// Inputs:
/// - `pending`: Request and active traversal path.
/// - `metadata`: Verified provider metadata.
/// - Resolution flags, bounds, and mutable graph state.
///
/// Output:
/// - Adds a graph node/edge and queues selected child requests when allowed.
///
/// Details:
/// - Cycles, depth limits, node limits, malformed metadata, and incompatible constraints become
///   diagnostics while sibling traversal continues in lexical order.
#[allow(clippy::too_many_arguments)]
fn process_found_metadata(
    pending: PendingRequest,
    metadata: DependencyMetadata,
    include_optdepends: bool,
    include_makedepends: bool,
    include_checkdepends: bool,
    nodes: &mut BTreeMap<String, DependencyGraphNode>,
    edges: &mut Vec<DependencyGraphEdge>,
    roots: &mut BTreeSet<String>,
    config: &DependencyGraphConfig,
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
    pending_requests: &mut Vec<PendingRequest>,
    expanded_depths: &mut BTreeMap<String, usize>,
) {
    let Some((data, selected_name)) = select_srcinfo_package(&metadata, &pending, diagnostics)
    else {
        let missing = missing_node(
            &pending.requested_name,
            pending.depth,
            Some(metadata.source.clone()),
        );
        if insert_node_if_allowed(missing, nodes, config, diagnostics) {
            add_edge(
                edges,
                pending.parent.as_deref(),
                &pending.requested_name,
                &pending.requested_name,
                &pending.version_req,
            );
            if pending.parent.is_none() {
                roots.insert(pending.requested_name);
            }
        }
        return;
    };
    let is_provider = selected_name != pending.requested_name;
    let node = DependencyGraphNode {
        name: selected_name.clone(),
        pkgbase: (!data.pkgbase.is_empty()).then_some(data.pkgbase.clone()),
        version: srcinfo_version(&data),
        provenance: DependencyProvenance {
            requested_name: pending.requested_name.clone(),
            source: Some(metadata.source),
            provider: is_provider.then_some(selected_name.clone()),
        },
        status: DependencyGraphNodeStatus::Resolved,
        constraints: DependencyConstraintRange::default(),
        provides: data
            .packages
            .iter()
            .find(|package| package.name == selected_name)
            .map_or_else(Vec::new, |package| package.provides.clone()),
        conflicts: data
            .packages
            .iter()
            .find(|package| package.name == selected_name)
            .map_or_else(Vec::new, |package| package.conflicts.clone()),
        depth: pending.depth,
    };
    if !insert_node_if_allowed(node, nodes, config, diagnostics) {
        return;
    }
    add_edge(
        edges,
        pending.parent.as_deref(),
        &selected_name,
        &pending.requested_name,
        &pending.version_req,
    );
    if pending.parent.is_none() {
        roots.insert(selected_name.clone());
    }
    let Some(node) = nodes.get_mut(&selected_name) else {
        return;
    };
    node.depth = node.depth.min(pending.depth);
    if !is_provider {
        merge_node_requirement(
            node,
            &pending.version_req,
            pending.parent.as_deref(),
            diagnostics,
        );
    }
    if pending.path.contains(&selected_name) {
        push_diagnostic(
            diagnostics,
            DependencyGraphDiagnosticKind::Cycle,
            &selected_name,
            pending.parent,
            "dependency cycle detected; branch expansion stopped",
        );
        return;
    }
    if !mark_expansion(expanded_depths, &selected_name, pending.depth) {
        return;
    }
    if pending.depth >= config.max_depth {
        let has_dependencies = data
            .packages
            .iter()
            .find(|package| package.name == selected_name)
            .is_some_and(|package| {
                !selected_dependencies(
                    package,
                    include_optdepends,
                    include_makedepends,
                    include_checkdepends,
                )
                .is_empty()
            });
        if has_dependencies {
            push_diagnostic(
                diagnostics,
                DependencyGraphDiagnosticKind::DepthLimit,
                &selected_name,
                None,
                format!(
                    "dependency graph depth limit ({}) reached",
                    config.max_depth
                ),
            );
        }
        return;
    }
    let Some(package) = data
        .packages
        .iter()
        .find(|package| package.name == selected_name)
    else {
        return;
    };
    let mut next_path = pending.path;
    next_path.push(selected_name.clone());
    for dependency in selected_dependencies(
        package,
        include_optdepends,
        include_makedepends,
        include_checkdepends,
    ) {
        let specification = parse_dep_spec(&dependency);
        if specification.name.is_empty() {
            continue;
        }
        pending_requests.push(PendingRequest {
            parent: Some(selected_name.clone()),
            requested_name: specification.name,
            version_req: specification.version_req,
            depth: pending.depth + 1,
            path: next_path.clone(),
        });
    }
}

/// What: Record the shallowest expansion depth for one selected package.
///
/// Inputs:
/// - `expanded_depths`: Package-to-shallowest-expanded-depth map.
/// - `package`: Selected package identity.
/// - `depth`: Incoming traversal depth.
///
/// Output:
/// - `true` when children must be expanded, `false` for an already-expanded equal/deeper path.
///
/// Details:
/// - Bounds traversal work by package/depth instead of the exponential number of distinct paths.
/// - A later shallower path is allowed to expand because it can reach children hidden by the
///   maximum-depth bound on an earlier path.
fn mark_expansion(
    expanded_depths: &mut BTreeMap<String, usize>,
    package: &str,
    depth: usize,
) -> bool {
    if expanded_depths
        .get(package)
        .is_some_and(|known_depth| *known_depth <= depth)
    {
        return false;
    }
    expanded_depths.insert(package.to_string(), depth);
    true
}

/// What: Match a declared conflict against one resolved package or virtual provider.
///
/// Inputs:
/// - `conflict`: Raw conflict specification.
/// - `candidate`: Other resolved graph node.
///
/// Output:
/// - Returns `true` when package or virtual identity and any version requirement match.
///
/// Details:
/// - Versioned virtual provides use their declared `=version`; unversioned provides only satisfy
///   unversioned conflicts, avoiding unverified version assumptions.
fn conflict_matches_node(conflict: &str, candidate: &DependencyGraphNode) -> bool {
    let conflict_spec = parse_dep_spec(conflict);
    if conflict_spec.name.is_empty() {
        return false;
    }
    if candidate.name == conflict_spec.name {
        return conflict_spec.version_req.is_empty()
            || candidate
                .version
                .as_deref()
                .is_some_and(|version| version_satisfies(version, &conflict_spec.version_req));
    }
    candidate.provides.iter().any(|provided| {
        let provided_spec = parse_dep_spec(provided);
        if provided_spec.name != conflict_spec.name {
            return false;
        }
        if conflict_spec.version_req.is_empty() {
            return true;
        }
        provided_spec
            .version_req
            .strip_prefix('=')
            .is_some_and(|version| version_satisfies(version, &conflict_spec.version_req))
    })
}

/// What: Mark mutually conflicting graph nodes and preserve conflict diagnostics.
///
/// Inputs:
/// - `nodes`: Resolved graph nodes indexed by package name.
/// - `diagnostics`: Resolution diagnostics.
///
/// Output:
/// - Updates conflicting node status and appends deterministic conflict events.
///
/// Details:
/// - The check works for official, local, and AUR metadata because it relies only on verified
///   injected source metadata and does not infer a source from missing system commands.
fn apply_conflicts(
    nodes: &mut BTreeMap<String, DependencyGraphNode>,
    diagnostics: &mut Vec<DependencyGraphDiagnostic>,
) {
    let names = nodes.keys().cloned().collect::<Vec<_>>();
    let mut matches = Vec::new();
    for (index, left_name) in names.iter().enumerate() {
        for right_name in names.iter().skip(index + 1) {
            let (Some(left), Some(right)) = (nodes.get(left_name), nodes.get(right_name)) else {
                continue;
            };
            if left.status != DependencyGraphNodeStatus::Resolved
                || right.status != DependencyGraphNodeStatus::Resolved
            {
                continue;
            }
            if left
                .conflicts
                .iter()
                .any(|conflict| conflict_matches_node(conflict, right))
                || right
                    .conflicts
                    .iter()
                    .any(|conflict| conflict_matches_node(conflict, left))
            {
                matches.push((left_name.clone(), right_name.clone()));
            }
        }
    }
    for (left_name, right_name) in matches {
        if let Some(left) = nodes.get_mut(&left_name) {
            left.status = DependencyGraphNodeStatus::Conflicting;
        }
        if let Some(right) = nodes.get_mut(&right_name) {
            right.status = DependencyGraphNodeStatus::Conflicting;
        }
        push_diagnostic(
            diagnostics,
            DependencyGraphDiagnosticKind::Conflict,
            left_name,
            Some(right_name),
            "declared package or virtual conflict matched another resolved graph node",
        );
    }
}

/// What: Resolve a bounded dependency graph through injected `.SRCINFO` metadata.
///
/// Inputs:
/// - `packages`: Root package references; their declared source is not treated as metadata proof.
/// - `provider`: Mockable provider returning verified `.SRCINFO` metadata in batches.
/// - `config`: Depth, node, timeout, and provider-batch bounds.
/// - Inclusion flags: Optional, make, and check dependency controls from the legacy resolver.
///
/// Output:
/// - Returns a deterministic graph with partial nodes and diagnostics for non-fatal branch errors.
///
/// Details:
/// - Metadata is cached for the duration of one call, traversed breadth-first in lexical order, and
///   requested in bounded serial batches. This function never executes pacman, an AUR helper, or
///   network I/O itself, keeping `deps` independent from the `aur` feature.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_dependency_graph<P: DependencyMetadataProvider>(
    packages: &[PackageRef],
    provider: &P,
    config: DependencyGraphConfig,
    include_optdepends: bool,
    include_makedepends: bool,
    include_checkdepends: bool,
) -> Result<DependencyGraphResolution> {
    validate_graph_config(&config)?;
    let mut pending_requests = packages
        .iter()
        .map(|package| PendingRequest {
            parent: None,
            requested_name: package.name.clone(),
            version_req: String::new(),
            depth: 0,
            path: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut cache = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    let mut roots = BTreeSet::new();
    let mut diagnostics = Vec::new();
    let mut expanded_depths = BTreeMap::new();

    while !pending_requests.is_empty() {
        sort_pending(&mut pending_requests);
        let request = pending_requests.remove(0);
        if !cache.contains_key(&request.requested_name) {
            let mut batch = Vec::new();
            for pending in std::iter::once(&request).chain(pending_requests.iter()) {
                if batch.len() == config.max_concurrency {
                    break;
                }
                if !cache.contains_key(&pending.requested_name)
                    && !batch.contains(&pending.requested_name)
                {
                    batch.push(pending.requested_name.clone());
                }
            }
            let started = Instant::now();
            let responses = provider.fetch_metadata(&batch, config.metadata_timeout);
            if started.elapsed() > config.metadata_timeout {
                for name in &batch {
                    push_diagnostic(
                        &mut diagnostics,
                        DependencyGraphDiagnosticKind::Timeout,
                        name,
                        None,
                        format!(
                            "metadata provider exceeded timeout of {:?}",
                            config.metadata_timeout
                        ),
                    );
                }
                cache_batch_responses(
                    &batch,
                    batch
                        .iter()
                        .map(|name| DependencyMetadataResponse::Failure {
                            requested_name: name.clone(),
                            message: "metadata provider timed out".to_string(),
                        })
                        .collect(),
                    &mut cache,
                    &mut diagnostics,
                );
            } else {
                cache_batch_responses(&batch, responses, &mut cache, &mut diagnostics);
            }
        }
        let Some(response) = cache.get(&request.requested_name).cloned() else {
            continue;
        };
        match response {
            DependencyMetadataResponse::Found(metadata) => process_found_metadata(
                request,
                metadata,
                include_optdepends,
                include_makedepends,
                include_checkdepends,
                &mut nodes,
                &mut edges,
                &mut roots,
                &config,
                &mut diagnostics,
                &mut pending_requests,
                &mut expanded_depths,
            ),
            unavailable => process_unavailable_metadata(
                &request,
                &unavailable,
                &mut nodes,
                &mut edges,
                &mut roots,
                &config,
                &mut diagnostics,
            ),
        }
    }

    apply_conflicts(&mut nodes, &mut diagnostics);
    edges.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| left.requested_name.cmp(&right.requested_name))
            .then_with(|| left.version_req.cmp(&right.version_req))
    });
    diagnostics.sort_by(|left, right| {
        format!("{:?}", left.kind)
            .cmp(&format!("{:?}", right.kind))
            .then_with(|| left.package.cmp(&right.package))
            .then_with(|| left.related_package.cmp(&right.related_package))
            .then_with(|| left.message.cmp(&right.message))
    });
    Ok(DependencyGraphResolution {
        roots: roots.into_iter().collect(),
        nodes: nodes.into_values().collect(),
        edges,
        diagnostics,
    })
}

impl DependencyGraphResolution {
    /// What: Render an already-resolved graph as a stable plain-text dependency tree.
    ///
    /// Inputs:
    /// - `self`: A graph result returned by `DependencyResolver::resolve_graph`.
    ///
    /// Output:
    /// - Returns a deterministic newline-terminated tree without performing metadata resolution.
    ///
    /// Details:
    /// - Children are lexical, shared subgraphs are shown under each parent, and active-path cycles
    ///   use `↺` rather than recursing indefinitely. Rendering does not mutate graph state.
    #[must_use]
    pub fn render_tree(&self) -> String {
        let mut output = String::new();
        for (index, root) in self.roots.iter().enumerate() {
            let mut path = BTreeSet::new();
            render_tree_node(
                self,
                root,
                "",
                true,
                index + 1 < self.roots.len(),
                &mut path,
                &mut output,
            );
        }
        output
    }
}

/// What: Render one tree node and its lexical descendants.
///
/// Inputs:
/// - Graph, node name, prefix state, active path, and output buffer.
///
/// Output:
/// - Appends the node and bounded descendants to the output buffer.
///
/// Details:
/// - This presentation helper consults only resolved graph data and uses an active-path set to
///   prevent cycle expansion while retaining deterministic shared-subgraph output.
#[allow(clippy::too_many_arguments)]
fn render_tree_node(
    graph: &DependencyGraphResolution,
    name: &str,
    prefix: &str,
    is_child: bool,
    has_next_root: bool,
    path: &mut BTreeSet<String>,
    output: &mut String,
) {
    let label = graph
        .nodes
        .iter()
        .find(|node| node.name == name)
        .map_or_else(
            || name.to_string(),
            |node| {
                if node.provenance.requested_name == node.name {
                    node.name.clone()
                } else {
                    format!("{} (for {})", node.name, node.provenance.requested_name)
                }
            },
        );
    if is_child {
        output.push_str(prefix);
        output.push_str(if has_next_root {
            "├── "
        } else {
            "└── "
        });
    }
    output.push_str(&label);
    output.push('\n');
    if !path.insert(name.to_string()) {
        output.push_str(prefix);
        output.push_str("    ↺\n");
        return;
    }
    let children = graph
        .edges
        .iter()
        .filter(|edge| edge.from == name)
        .map(|edge| edge.to.as_str())
        .collect::<BTreeSet<_>>();
    for (index, child) in children.iter().enumerate() {
        let child_prefix = if is_child {
            format!("{prefix}{}", if has_next_root { "│   " } else { "    " })
        } else {
            String::new()
        };
        render_tree_node(
            graph,
            child,
            &child_prefix,
            true,
            index + 1 < children.len(),
            path,
            output,
        );
    }
    path.remove(name);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What: Verify epoch and pkgrel-aware constraint intersection.
    ///
    /// Inputs:
    /// - Static compatible and incompatible requirement strings.
    ///
    /// Output:
    /// - Confirms lower and upper bounds use full package version semantics.
    ///
    /// Details:
    /// - The test is pure and does not require host package metadata or a provider.
    #[test]
    fn intersect_requirement_handles_epoch_and_pkgrel() {
        let range = intersect_requirement(&DependencyConstraintRange::default(), ">=1:2.0-3")
            .unwrap_or_default();
        let range = intersect_requirement(&range, "<=1:3.0-1").unwrap_or_default();
        assert_eq!(
            range.lower.as_ref().map(|bound| bound.version.as_str()),
            Some("1:2.0-3")
        );
        assert_eq!(
            range.upper.as_ref().map(|bound| bound.version.as_str()),
            Some("1:3.0-1")
        );
        assert!(intersect_requirement(&range, "<1:2.0-3").is_none());
    }

    /// What: Verify graph configuration rejects disabled safety bounds.
    ///
    /// Inputs:
    /// - Default config with each mandatory bound set to zero in turn.
    ///
    /// Output:
    /// - Confirms invalid bounds return errors before provider I/O.
    ///
    /// Details:
    /// - A zero depth remains valid because it intentionally permits root-only metadata resolution.
    #[test]
    fn validate_graph_config_rejects_zero_mandatory_bounds() {
        assert!(
            validate_graph_config(&DependencyGraphConfig {
                max_nodes: 0,
                ..DependencyGraphConfig::default()
            })
            .is_err()
        );
        assert!(
            validate_graph_config(&DependencyGraphConfig {
                metadata_timeout: Duration::ZERO,
                ..DependencyGraphConfig::default()
            })
            .is_err()
        );
        assert!(
            validate_graph_config(&DependencyGraphConfig {
                max_concurrency: 0,
                ..DependencyGraphConfig::default()
            })
            .is_err()
        );
    }

    /// What: Verify shared graph nodes expand once unless later reached by a shallower path.
    ///
    /// Inputs:
    /// - Repeated package/depth pairs against one expansion map.
    ///
    /// Output:
    /// - Equal/deeper repeats are skipped; a shallower repeat is accepted once.
    ///
    /// Details:
    /// - Prevents traversal work from scaling with the number of paths through dense shared graphs.
    #[test]
    fn expansion_memoization_bounds_shared_paths() {
        let mut expanded = BTreeMap::new();
        assert!(mark_expansion(&mut expanded, "shared", 4));
        assert!(!mark_expansion(&mut expanded, "shared", 4));
        assert!(!mark_expansion(&mut expanded, "shared", 6));
        assert!(mark_expansion(&mut expanded, "shared", 2));
        assert!(!mark_expansion(&mut expanded, "shared", 3));
    }

    /// What: Verify malformed requirements are distinguished from valid incompatible intervals.
    ///
    /// Inputs:
    /// - Empty, valid operator-prefixed, operator-less, and empty-version requirements.
    ///
    /// Output:
    /// - Only syntax accepted by interval intersection is reported well formed.
    ///
    /// Details:
    /// - Keeps actionable metadata diagnostics from mislabeling malformed input as a range conflict.
    #[test]
    fn requirement_validation_distinguishes_malformed_constraints() {
        assert!(requirement_is_well_formed(""));
        assert!(requirement_is_well_formed(">=1.0"));
        assert!(!requirement_is_well_formed("1.0"));
        assert!(!requirement_is_well_formed(">="));
    }
}

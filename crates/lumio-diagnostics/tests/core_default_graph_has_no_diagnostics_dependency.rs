//! T-diagnostics-04 / R-00177: default graph has no diagnostics dependency.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lumio_diagnostics::{BoundedRecorder, DiagnosticsResource, KernelRecordRef, RecordDisposition};
use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::context::{CancelReason, ContextConfig, ContextResource, KernelContext};
use lumio_platform::Deadline;

const CORE_CRATES: &[&str] = &[
    "lumio-kernel",
    "lumio-job",
    "lumio-platform",
    "lumio-contract-types",
    "lumio-spatial",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn crate_manifest(name: &str) -> PathBuf {
    workspace_root()
        .join("crates")
        .join(name)
        .join("Cargo.toml")
}

fn read_manifest(name: &str) -> String {
    let path = crate_manifest(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn section_body<'a>(text: &'a str, heading: &str) -> &'a str {
    let header = format!("[{heading}]");
    let Some(pos) = text.find(&header) else {
        return "";
    };
    let rest = &text[pos + header.len()..];
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
        .unwrap_or(rest);
    match rest.find("\n[") {
        Some(end) => {
            let body = &rest[..end];
            body.strip_suffix('\r').unwrap_or(body)
        }
        None => rest,
    }
}

fn is_package_key(line: &str, package: &str) -> bool {
    let line = line.trim();
    if line.starts_with('#') || !line.starts_with(package) {
        return false;
    }
    let rest = &line[package.len()..];
    rest.is_empty()
        || rest.starts_with('=')
        || rest.starts_with('.')
        || rest.starts_with(' ')
        || rest.starts_with('\t')
}

fn mentions_package(section: &str, package: &str) -> bool {
    section.lines().any(|line| is_package_key(line, package))
}

fn parse_string_list(line: &str) -> Vec<String> {
    let Some(start) = line.find('[') else {
        return Vec::new();
    };
    let Some(end) = line.rfind(']') else {
        return Vec::new();
    };
    if end < start {
        return Vec::new();
    }
    line[start + 1..end]
        .split(',')
        .map(|item| item.trim().trim_matches('"').trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn feature_values(features: &str, name: &str) -> Vec<String> {
    for line in features.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if is_package_key(line, name) {
            return parse_string_list(line);
        }
    }
    Vec::new()
}

fn default_enables_experimental_diagnostics(text: &str) -> bool {
    let features = section_body(text, "features");
    if features.is_empty() {
        return false;
    }
    let mut stack = feature_values(features, "default");
    let mut seen = BTreeSet::new();
    while let Some(feat) = stack.pop() {
        if !seen.insert(feat.clone()) {
            continue;
        }
        if feat == "experimental-diagnostics" || feat == "dep:lumio-diagnostics" {
            return true;
        }
        stack.extend(feature_values(features, &feat));
    }
    false
}

fn assert_no_normal_diagnostics_dep(name: &str, text: &str) {
    assert!(
        !mentions_package(section_body(text, "dependencies"), "lumio-diagnostics"),
        "{name} must not list lumio-diagnostics as a normal dependency"
    );
    assert!(
        section_body(text, "dependencies.lumio-diagnostics")
            .trim()
            .is_empty(),
        "{name} must not list [dependencies.lumio-diagnostics]"
    );
}

fn assert_default_does_not_enable_diagnostics(name: &str, text: &str) {
    assert!(
        !default_enables_experimental_diagnostics(text),
        "{name} default features must not enable experimental-diagnostics"
    );
}

fn test_config() -> ContextConfig {
    ContextConfig {
        limits: ConfiguredLimits {
            max_handles: 4,
            max_native_bytes: 64,
            max_jobs_queued: 1,
            max_jobs_running: 1,
            max_completion_items: 1,
        },
        quiesce_deadline: Deadline::NONE,
    }
}

fn try_record(resource: &DiagnosticsResource, kind: &str, payload: &[u8]) -> RecordDisposition {
    let fields = [kind];
    resource.try_record(KernelRecordRef {
        fields: &fields,
        payload,
    })
}

#[test]
fn core_default_graph_has_no_diagnostics_dependency() {
    for name in CORE_CRATES {
        let text = read_manifest(name);
        assert_no_normal_diagnostics_dep(name, &text);
        assert_default_does_not_enable_diagnostics(name, &text);
    }
}

#[test]
fn diagnostics_resource_close_drops_late_record() {
    let resource = Arc::new(DiagnosticsResource::new(
        BoundedRecorder::with_capacity(4, 32).expect("valid recorder"),
    ));
    assert_eq!(
        try_record(&resource, "a", b"1"),
        RecordDisposition::Accepted
    );

    let ctx = KernelContext::create_for_test(test_config());
    let registration = ctx
        .register_resource(Arc::clone(&resource) as Arc<dyn ContextResource>)
        .expect("register diagnostics");
    assert_eq!(registration.name, "diagnostics");

    let port: &dyn ContextResource = resource.as_ref();
    assert_eq!(port.name(), "diagnostics");

    ctx.close(CancelReason::ContextClosing, Deadline::NONE)
        .expect("close");

    let late = try_record(&resource, "b", b"22");
    assert!(
        late == RecordDisposition::DroppedFull,
        "late try_record after destroy must be DroppedFull or a closed disposition, got {late:?}"
    );
}

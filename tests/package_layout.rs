use std::fs;
use std::path::{Path, PathBuf};

fn source(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("release-layout source must be readable")
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(directory).expect("source directory must be readable") {
        let entry = entry.expect("source directory entry must be readable");
        let path = entry.path();
        if path.is_dir() {
            sources.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    sources
}

#[test]
fn public_package_owns_runtime_and_retained_g36_binaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = source(root.join("Cargo.toml"));

    assert!(manifest.contains("name = \"hookstat-ipc-broker\""));
    assert!(manifest.contains("name = \"hookstat-hook\""));
    assert!(manifest.contains("path = \"src/bin/hookstat_hook.rs\""));
    assert!(manifest.contains("name = \"hookstat-shim-fixture\""));
    assert!(manifest.contains("required-features = [\"test-fixtures\"]"));
    assert!(!manifest.contains("hookstat-ipc-client"));
    assert!(manifest.contains("exclude = [\"dev_proof/**\"]"));
    assert!(!root.join("crates/hookstat-ipc-client/Cargo.toml").exists());
    assert!(!root.join("crates/hookstat-hook/Cargo.toml").exists());
}

#[test]
fn packaged_transparent_shim_is_explicitly_non_production_for_v031() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let admission = source(root.join("src/admission.rs"));
    let shim_binary = source(root.join("src/bin/hookstat_hook.rs"));

    assert!(admission.contains("V031_TRANSPARENT_SHIM_ADMISSION"));
    assert!(admission.contains("QualifiedNotAdmittedPerformance"));
    assert!(shim_binary.contains("--admission-status"));
    assert!(shim_binary.contains("not production activated for v0.3.1"));
}

#[test]
fn shim_reuses_internal_protocol_without_product_hot_path_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let shim = source(root.join("src/hook_shim.rs")).to_ascii_lowercase();
    let shim_binary = source(root.join("src/bin/hookstat_hook.rs"));

    for forbidden in ["ratatui", "crossterm", "rusqlite"] {
        assert!(
            !shim.contains(forbidden),
            "shim source must not acquire product hot-path dependency: {forbidden}"
        );
    }
    for forbidden in [
        "crate::analytics",
        "crate::workbench",
        "crate::report",
        "crate::tui::localization",
    ] {
        assert!(
            !shim.contains(forbidden),
            "shim source must not initialize product state: {forbidden}"
        );
    }
    assert!(shim_binary.contains("#[path = \"../ipc_client.rs\"]"));
    assert!(shim_binary.contains("#[path = \"../hook_shim.rs\"]"));
    assert!(root.join("src/ipc_client.rs").is_file());

    let broker = source(root.join("src/ipc.rs"));
    assert!(broker.contains("pub use crate::ipc_client"));
    assert!(broker.contains("IPC_MAGIC, IPC_PROTOCOL_VERSION"));

    let magic_definitions = rust_sources(&root.join("src"))
        .into_iter()
        .filter(|path| source(path).contains("*b\"HSIP\""))
        .map(|path| {
            path.strip_prefix(root.join("src"))
                .expect("source path must be rooted in src")
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    assert_eq!(magic_definitions, vec![PathBuf::from("ipc_client.rs")]);
}

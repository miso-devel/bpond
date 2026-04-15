/// Invariant tests: verify that all project name references use "bpond".
///
/// These tests enforce the rename from "mini-pond"/"terminal-zoo" to "bpond"
/// across every file that contains the old names.
use std::fs;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

// -- Cargo.toml --------------------------------------------------------------

#[test]
fn cargo_toml_package_name_is_bpond() {
    // Given: Cargo.toml at project root
    let content = read("Cargo.toml");

    // When/Then: package name must be "bpond"
    assert!(
        content.contains("name = \"bpond\""),
        "Cargo.toml must have `name = \"bpond\"`, got:\n{content}"
    );
}

#[test]
fn cargo_toml_has_no_mini_pond() {
    let content = read("Cargo.toml");
    assert!(
        !content.contains("mini-pond"),
        "Cargo.toml must not contain \"mini-pond\""
    );
}

// -- src/main.rs -------------------------------------------------------------

#[test]
fn main_rs_debug_header_uses_bpond() {
    // Given: draw_header format string in main.rs
    let content = read("src/main.rs");

    // When/Then: the header prefix must be "bpond", not "mini-pond"
    assert!(
        content.contains("\"  bpond  "),
        "src/main.rs debug header must start with \"  bpond  \""
    );
}

#[test]
fn main_rs_has_no_mini_pond() {
    let content = read("src/main.rs");
    assert!(
        !content.contains("mini-pond"),
        "src/main.rs must not contain \"mini-pond\""
    );
}

// -- Makefile ----------------------------------------------------------------

#[test]
fn makefile_debug_binary_path_is_bpond() {
    // Given: Makefile run target references the debug binary
    let content = read("Makefile");

    // When/Then: binary path must point to "bpond"
    assert!(
        content.contains("./target/debug/bpond"),
        "Makefile must reference ./target/debug/bpond"
    );
}

#[test]
fn makefile_release_binary_path_is_bpond() {
    let content = read("Makefile");
    assert!(
        content.contains("./target/release/bpond"),
        "Makefile must reference ./target/release/bpond"
    );
}

#[test]
fn makefile_has_no_mini_pond() {
    let content = read("Makefile");
    assert!(
        !content.contains("mini-pond"),
        "Makefile must not contain \"mini-pond\""
    );
}

// -- CLAUDE.md ---------------------------------------------------------------

#[test]
fn claude_md_heading_is_bpond() {
    // Given: CLAUDE.md top-level heading identifies the project
    let content = read("CLAUDE.md");

    // When/Then: first heading must be "# bpond"
    assert!(
        content.starts_with("# bpond"),
        "CLAUDE.md must start with \"# bpond\""
    );
}

#[test]
fn claude_md_has_no_mini_pond() {
    let content = read("CLAUDE.md");
    assert!(
        !content.contains("mini-pond"),
        "CLAUDE.md must not contain \"mini-pond\""
    );
}

// -- README.md ---------------------------------------------------------------

#[test]
fn readme_has_no_mini_pond() {
    let content = read("README.md");
    assert!(
        !content.contains("mini-pond"),
        "README.md must not contain \"mini-pond\""
    );
}

#[test]
fn readme_contains_bpond() {
    let content = read("README.md");
    assert!(
        content.contains("bpond"),
        "README.md must contain \"bpond\""
    );
}

// -- .claude/launch.json -----------------------------------------------------

#[test]
fn launch_json_has_no_terminal_zoo() {
    let content = read(".claude/launch.json");
    assert!(
        !content.contains("terminal-zoo"),
        ".claude/launch.json must not contain \"terminal-zoo\""
    );
}

#[test]
fn launch_json_debug_config_name_is_bpond() {
    // Given: launch.json defines a debug configuration
    let content = read(".claude/launch.json");

    // When/Then: configuration name must reference "bpond"
    assert!(
        content.contains("bpond (debug)"),
        ".claude/launch.json must contain \"bpond (debug)\""
    );
}

// -- demo.tape ---------------------------------------------------------------

#[test]
fn demo_tape_has_no_terminal_zoo() {
    let content = read("demo.tape");
    assert!(
        !content.contains("terminal-zoo"),
        "demo.tape must not contain \"terminal-zoo\""
    );
}

// -- directory name ----------------------------------------------------------

#[test]
fn project_directory_name_is_bpond() {
    // Given: the project root directory
    let cwd = std::env::current_dir().expect("failed to get current dir");

    // When/Then: the directory basename must be "bpond"
    let dirname = cwd
        .file_name()
        .expect("current dir has no file name")
        .to_str()
        .expect("directory name is not valid UTF-8");
    assert_eq!(
        dirname, "bpond",
        "project directory must be named \"bpond\", got \"{dirname}\""
    );
}

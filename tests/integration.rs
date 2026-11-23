use std::fs;

use cargo_test_support::{
    Project, cargo_test, paths,
    prelude::IntoData,
    project,
    registry::{Package, registry_path},
    str,
};

#[cargo_test]
fn available_update() {
    let project = locked_project();
    Package::new("example-dep", "1.0.1").local(true).publish();

    assert_output(
        &project,
        str![[r#"
Name         Project  Compat  Latest  Kind    Platform
----         -------  ------  ------  ----    --------
example-dep  1.0.0    1.0.1   1.0.1   Normal  ---

"#]],
    );
}

#[cargo_test]
fn no_update_required() {
    let project = locked_project();

    assert_output(
        &project,
        str![[r#"
All dependencies are up to date, yay!

"#]],
    );
}

#[cargo_test]
fn workspace_dependency_update() {
    local_registry();
    Package::new("example-dep", "1.0.0").local(true).publish();

    let mut builder = project().file(
        "Cargo.toml",
        r#"
[workspace]
resolver = "2"
members = ["one", "two"]

[workspace.dependencies]
example-dep = "1.0"
"#,
    );
    for name in ["one", "two"] {
        builder = builder
            .file(
                format!("{name}/Cargo.toml"),
                &format!(
                    r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
example-dep.workspace = true
"#,
                ),
            )
            .file(format!("{name}/src/lib.rs"), "");
    }
    let project = builder.build();
    generate_lockfile(&project);
    Package::new("example-dep", "1.0.1").local(true).publish();

    assert_output(
        &project,
        str![[r#"
Workspace
================
Name         Project  Compat  Latest  Kind    Platform
----         -------  ------  ------  ----    --------
example-dep  1.0.0    ---     1.0.1   Normal  ---

"#]],
    );
}

#[cargo_test]
fn inherited_dependency_update() {
    local_registry();
    Package::new("runtime", "1.0.0").local(true).publish();
    Package::new("generator", "1.0.0")
        .dep("runtime", "^1.0.0")
        .local(true)
        .publish();

    let project = project()
        .file(
            "Cargo.toml",
            r#"
[workspace]
members = ["member"]
resolver = "2"

[workspace.dependencies]
runtime = "=1.0.0"
"#,
        )
        .file(
            "member/Cargo.toml",
            r#"
[package]
name = "member"
version = "0.1.0"
edition = "2021"

[dependencies]
runtime = { workspace = true }
generator = "=1.0.0"
"#,
        )
        .file("member/src/lib.rs", "")
        .build();
    generate_lockfile(&project);

    Package::new("runtime", "1.0.1").local(true).publish();
    Package::new("generator", "1.0.1")
        .dep("runtime", "^1.0.1")
        .local(true)
        .publish();

    assert_output(
        &project,
        str![[r#"
Workspace
================
Name     Project  Compat  Latest  Kind    Platform
----     -------  ------  ------  ----    --------
runtime  1.0.0    ---     1.0.1   Normal  ---

member
================
Name       Project  Compat  Latest  Kind    Platform
----       -------  ------  ------  ----    --------
generator  1.0.0    ---     1.0.1   Normal  ---

"#]],
    );
}

fn locked_project() -> Project {
    local_registry();
    Package::new("example-dep", "1.0.0").local(true).publish();

    let project = project()
        .file(
            "Cargo.toml",
            r#"
[package]
name = "example"
version = "0.1.0"
edition = "2021"

[dependencies]
example-dep = "1.0"
"#,
        )
        .file("src/lib.rs", "")
        .build();

    generate_lockfile(&project);
    project
}

fn generate_lockfile(project: &Project) {
    project
        .process(env!("CARGO"))
        .args(&["generate-lockfile", "--offline"])
        .run();
}

fn assert_output(project: &Project, stdout: impl IntoData) {
    let output = project
        .process(env!("CARGO_BIN_EXE_cargo-outdated"))
        .args(&["outdated", "--offline", "--depth", "1", "--color", "never"])
        .with_stdout_data(stdout)
        .run();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr
            .lines()
            .all(|line| cfg!(feature = "debug") && line.trim_start().starts_with("DEBUG ")),
        "unexpected stderr:\n{stderr}"
    );
}

fn local_registry() {
    fs::create_dir_all(paths::cargo_home()).unwrap();
    let registry = toml::Value::String(registry_path().to_str().unwrap().to_owned());
    fs::write(
        paths::cargo_home().join("config.toml"),
        format!(
            r#"
[source.crates-io]
replace-with = "test-registry"

[source.test-registry]
local-registry = {registry}
"#
        ),
    )
    .unwrap();
}

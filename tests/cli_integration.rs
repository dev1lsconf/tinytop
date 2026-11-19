use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tinytop"))
}

#[test]
fn check_config_deep_validates_db_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("config.toml");
    let db_path = dir.path().join("nested/tinytop.db");
    std::fs::write(
        &cfg_path,
        format!(
            "bind = \"127.0.0.1\"\nport = 19096\ndb_path = \"{}\"\ninterval_seconds = 30\nretention_hours = 24\nmax_history_hours = 24\nrequest_timeout_secs = 5\n\n[[targets]]\nname = \"local\"\nurl = \"http://127.0.0.1:1/\"\n",
            db_path.display()
        ),
    )
    .expect("write config");

    let out = Command::new(bin())
        .args([
            "--check-config",
            "--deep",
            "--config",
            cfg_path.to_str().expect("utf8"),
        ])
        .output()
        .expect("run tinytop");
    assert!(
        out.status.success(),
        "status={:?} stdout={} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("config ok (deep):"), "stdout={stdout}");
    assert!(
        stdout.contains("db open/migrate/ping ok"),
        "stdout={stdout}"
    );
    assert!(
        db_path.exists(),
        "deep check should create/open db_path {db_path:?}"
    );
}

#[test]
fn cli_rejects_unknown_argument() {
    let out = Command::new(bin())
        .arg("--bogus")
        .output()
        .expect("run tinytop");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown argument"), "stderr={stderr}");
    assert!(stderr.contains("--bogus"), "stderr={stderr}");
}

#[test]
fn cli_requires_config_path_value() {
    let out = Command::new(bin())
        .arg("--config")
        .output()
        .expect("run tinytop");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--config requires a path"),
        "stderr={stderr}"
    );
}

#[test]
fn cli_rejects_deep_without_check_config() {
    let out = Command::new(bin())
        .arg("--deep")
        .output()
        .expect("run tinytop");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--deep requires --check-config"),
        "stderr={stderr}"
    );
}

#[test]
fn check_config_deep_fails_when_db_path_unopenable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("config.toml");
    // Use an existing directory as db path: sqlite cannot open it as a database file.
    let db_path = dir.path().join("not-a-db-file");
    std::fs::create_dir_all(&db_path).expect("create dir as db path");
    std::fs::write(
        &cfg_path,
        format!(
            "bind = \"127.0.0.1\"\nport = 19097\ndb_path = \"{}\"\ninterval_seconds = 30\nretention_hours = 24\nmax_history_hours = 24\nrequest_timeout_secs = 5\n\n[[targets]]\nname = \"local\"\nurl = \"http://127.0.0.1:1/\"\n",
            db_path.display()
        ),
    )
    .expect("write config");

    let out = Command::new(bin())
        .args([
            "--check-config",
            "--deep",
            "--config",
            cfg_path.to_str().expect("utf8"),
        ])
        .output()
        .expect("run tinytop");
    assert!(
        !out.status.success(),
        "deep check should fail for unopenable db_path, stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn check_config_validates_example_configs() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "examples/config.basic.toml",
        "examples/config.advanced.toml",
    ] {
        let cfg_path = manifest.join(rel);
        let out = Command::new(bin())
            .args([
                "--check-config",
                "--config",
                cfg_path.to_str().expect("utf8"),
            ])
            .output()
            .expect("run tinytop");
        assert!(
            out.status.success(),
            "example {} should be valid, status={:?} stdout={} stderr={}",
            rel,
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("config ok:"), "stdout={stdout}");
    }
}

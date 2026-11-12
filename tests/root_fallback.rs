//! nt fix (`NIU_ROOT_FALLBACK`): `/` maps to NIU_ROOT, so `cd /src` failed
//! although `C:\src` exists. With the opt-in fallback an absolute POSIX
//! path whose first component is missing below NIU_ROOT resolves against
//! the SystemDrive root (rubash `root_fallback_path`). `Windows` stands in
//! for `src`: it exists at every system drive root and never below a
//! fresh shell root.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn niu_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_niu"))
}

fn run(script: &str, root: &Path, fallback: Option<&str>) -> Output {
    let mut command = Command::new(niu_binary());
    command
        .arg("--norc")
        .arg("-c")
        .arg(script)
        .env("NIU_ROOT", root)
        .env_remove("NIU_ROOT_FALLBACK")
        .env_remove("RUBASH_ROOT_FALLBACK")
        .stdin(Stdio::null());
    if let Some(value) = fallback {
        command.env("NIU_ROOT_FALLBACK", value);
    }
    command
        .output()
        .unwrap_or_else(|err| panic!("spawn niu: {err}"))
}

fn unique_root(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(root.join("etc")).unwrap();
    root
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

#[test]
fn without_fallback_root_children_stay_below_niu_root() {
    let root = unique_root("niu-root-fallback-off");
    let output = run("cd /Windows", &root, None);
    assert!(
        !output.status.success(),
        "cd /Windows must fail below a fresh NIU_ROOT without the fallback"
    );
    let output = run("cd /Windows", &root, Some("0"));
    assert!(!output.status.success(), "NIU_ROOT_FALLBACK=0 means off");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn fallback_resolves_cd_test_and_redirects_on_system_drive() {
    let root = unique_root("niu-root-fallback-on");
    let script = "cd /Windows/System32 && echo cd-ok; \
                  [ -d /Windows/System32 ] && echo test-ok; \
                  [ -d /etc ] && echo etc-ok; \
                  read -r _ < /Windows/win.ini && echo redirect-ok; \
                  cd / && [ -d etc ] && echo root-ok";
    let output = run(script, &root, Some("1"));
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    assert_eq!(
        stdout_of(&output)
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>(),
        ["cd-ok", "test-ok", "etc-ok", "redirect-ok", "root-ok"],
        "stderr: {}",
        stderr_of(&output)
    );
    let _ = std::fs::remove_dir_all(root);
}

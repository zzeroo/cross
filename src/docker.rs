use std::{env, fs};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use Target;
use errors::*;
use extensions::CommandExt;
use id;
use rustc;

/// Register QEMU interpreters
pub fn register(verbose: bool) -> Result<()> {
    Command::new("docker")
        .arg("run")
        .arg("--privileged")
        .arg("--rm")
        .arg("-it")
        .arg("ubuntu:16.04")
        .args(&["sh",
                "-c",
                "apt-get update && apt-get install --no-install-recommends \
                 -y binfmt-support qemu-user-static"])
        .run(verbose)
}

pub fn run(target: Target,
           args: &[String],
           cargo_root: &Path,
           verbose: bool)
           -> Result<ExitStatus> {
    let home_dir = env::home_dir()
        .ok_or_else(|| "couldn't get home directory. Is $HOME not set?")?;
    let cargo_dir = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join(".cargo"));
    let xargo_dir = env::var_os("XARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join(".xargo"));
    let target_dir = cargo_root.join("target");

    // create the directories we are going to mount before we mount them,
    // otherwise `docker` will create them but they will be owned by `root`
    fs::create_dir(&target_dir).ok();
    fs::create_dir(&cargo_dir).ok();
    fs::create_dir(&xargo_dir).ok();

    let mut cargo = if target.uses_xargo() {
        Command::new("xargo")
    } else {
        Command::new("cargo")
    };
    cargo.args(args);

    let version = env!("CARGO_PKG_VERSION");
    let tag = if version.ends_with("-dev") {
        Cow::from("latest")
    } else {
        Cow::from(format!("v{}", version))
    };

    let mut docker = Command::new("docker");

    docker.arg("run")
        .arg("--rm")
        .args(&["--user", &format!("{}:{}", id::user(), id::group())])
        .args(&["-e", "CARGO_HOME=/cargo"])
        .args(&["-e", "CARGO_TARGET_DIR=/target"])
        .args(&["-e", "HOME=/tmp"])
        .args(&["-e", &format!("USER={}", id::username())])
        .args(&["-e", "XARGO_HOME=/xargo"])
        .args(&["-v", &format!("{}:/xargo", xargo_dir.display())])
        .args(&["-v", &format!("{}:/cargo", cargo_dir.display())])
        .args(&["-v", &format!("{}:/project:ro", cargo_root.display())])
        .args(&["-v",
                &format!("{}:/rust:ro", rustc::sysroot(verbose)?.display())])
        .args(&["-v", &format!("{}:/target", target_dir.display())]);

    // `emcc` caches stuff in `$HOME`. We must preserve that information across
    // runs so we use an OS-level temporary directory for that.
    let must_run_emcc_first = if target.uses_emcc() {
        let temp_dir = env::temp_dir();
        let cache_dir = temp_dir.join(format!("cross-{}", target.triple()));

        if !cache_dir.exists() {
            fs::create_dir(&cache_dir).chain_err(|| {
                    format!("couldn't create a directory in {}",
                            temp_dir.display())
                })?;
        }

        docker.args(&["-v", &format!("{}:{}", cache_dir.display(), "/tmp")]);

        !cache_dir.join(".emscripten").exists()
    } else {
        false
    };

    docker.args(&["-w", "/project"])
        .args(&["-it", &format!("japaric/{}:{}", target.triple(), tag)]);

    // `emcc` only creates the `~/.emscripten` file the very first time is
    // called. Make sure that first time is from "outside" `rustc` so that
    // `cross build` doesn't result in a no-op.
    if must_run_emcc_first {
        docker.args(&["sh",
                      "-c",
                      &format!("emcc 2>/dev/null; \
                                PATH=$PATH:/rust/bin {:?}",
                               cargo)]);
    } else {
        docker.args(&["sh",
                      "-c",
                      &format!("PATH=$PATH:/rust/bin {:?}", cargo)]);
    }

    docker.run_and_get_status(verbose)
}

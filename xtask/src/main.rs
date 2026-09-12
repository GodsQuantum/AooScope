use aooscope_config::AppPaths;
use aooscope_server::{ApiDoc, AppState, app};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use utoipa::OpenApi;

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("api-schema") => {
            let path = args.next().ok_or("usage: xtask api-schema <path>")?;
            write_if_changed(PathBuf::from(path), &ApiDoc::openapi())
        }
        Some("check") => check(),
        Some("dist") => dist(),
        Some("parity-server") => {
            let config = args
                .next()
                .ok_or("usage: xtask parity-server <config-dir> <bind>")?;
            let bind = args
                .next()
                .ok_or("usage: xtask parity-server <config-dir> <bind>")?;
            parity_server(PathBuf::from(config), &bind)
        }
        Some(command) => Err(format!("unknown command: {command}")),
        None => Err("missing command".into()),
    }
}

fn parity_server(config: PathBuf, bind: &str) -> Result<(), String> {
    let address: std::net::SocketAddr = bind
        .parse()
        .map_err(|e| format!("invalid bind address: {e}"))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|e| e.to_string())?;
        let state = AppState::new(AppPaths::new(config.clone()))
            .with_device(config.join(".parity-device-does-not-exist"));
        eprintln!(
            "parity-server ready on {}",
            listener.local_addr().map_err(|e| e.to_string())?
        );
        axum::serve(listener, app(state))
            .await
            .map_err(|e| e.to_string())
    })
}

fn root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or("xtask has no workspace parent".into())
}

fn check() -> Result<(), String> {
    let root = root()?;
    let frontend = root.join("frontend");
    run_cmd(
        &frontend,
        "npm",
        &["exec", "--yes", "pnpm@12.4.1", "--", "check"],
    )?;
    run_cmd(
        &frontend,
        "npm",
        &["exec", "--yes", "pnpm@12.4.1", "--", "test", "--run"],
    )?;
    run_cmd(
        &frontend,
        "npm",
        &["exec", "--yes", "pnpm@12.4.1", "--", "build"],
    )?;
    run_cmd(&root, "cargo", &["fmt", "--all", "--", "--check"])?;
    run_cmd(
        &root,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    let nextest = Command::new("cargo")
        .args(["nextest", "--version"])
        .output()
        .is_ok_and(|output| output.status.success());
    if nextest {
        run_cmd(&root, "cargo", &["nextest", "run", "--workspace"])?;
    } else {
        eprintln!("xtask: cargo-nextest unavailable; falling back to cargo test");
        run_cmd(&root, "cargo", &["test", "--workspace"])?;
    }
    run_cmd(
        &root,
        "python3",
        &["-m", "unittest", "tests.test_repo_hygiene", "-v"],
    )?;
    Ok(())
}

fn dist() -> Result<(), String> {
    let root = root()?;
    run_cmd(
        &root.join("frontend"),
        "npm",
        &["exec", "--yes", "pnpm@12.4.1", "--", "build"],
    )?;
    run_cmd(
        &root,
        "cargo",
        &["build", "--release", "--locked", "-p", "aooscope-server"],
    )?;
    let out = root.join("dist");
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    fs::copy(
        root.join("target/release/aooscope-server"),
        out.join("aooscope"),
    )
    .map_err(|e| e.to_string())?;
    write_if_changed(out.join("openapi.json"), &ApiDoc::openapi())
}

fn run_cmd(cwd: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    eprintln!("+ (cd {} && {} {})", cwd.display(), program, args.join(" "));
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd);
    if program == "cargo" {
        if command_exists("sccache") {
            command.env("RUSTC_WRAPPER", "sccache");
        }
        if cfg!(target_os = "linux") && command_exists("mold") {
            let existing = env::var("RUSTFLAGS").unwrap_or_default();
            let mold = "-C link-arg=-fuse-ld=mold";
            command.env("RUSTFLAGS", format!("{existing} {mold}").trim());
        }
    }
    let status = command
        .status()
        .map_err(|e| format!("cannot run {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

fn command_exists(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn write_if_changed(path: PathBuf, doc: &utoipa::openapi::OpenApi) -> Result<(), String> {
    let mut content = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    content.push('\n');
    if fs::read_to_string(&path).ok().as_deref() == Some(&content) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| e.to_string())
}

use aooscope_server::ApiDoc;
use std::{env, fs, path::PathBuf};
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
        Some(command) => Err(format!("unknown command: {command}")),
        None => Err("missing command".into()),
    }
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

use protocol::FromRunner;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

const WORK: &str = "/work";

fn resolve(path: &str) -> Result<PathBuf, FromRunner> {
    let rel = Path::new(path);
    if path.is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(FromRunner::Error {
            message: format!("path must be relative to {WORK} without '.' or '..': {path:?}"),
        });
    }
    Ok(Path::new(WORK).join(rel))
}

pub fn put(path: &str, bytes: &[u8]) -> FromRunner {
    let target = match resolve(path) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match fs::write(target, bytes) {
        Ok(()) => FromRunner::FileOk,
        Err(e) => FromRunner::Error {
            message: e.to_string(),
        },
    }
}

pub fn stat(path: &str) -> FromRunner {
    let target = match resolve(path) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match fs::metadata(target) {
        Ok(m) => FromRunner::Stat {
            exists: true,
            size: m.len(),
        },
        Err(e) if e.kind() == ErrorKind::NotFound => FromRunner::Stat {
            exists: false,
            size: 0,
        },
        Err(e) => FromRunner::Error {
            message: e.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_relative_paths_inside_work() {
        assert_eq!(resolve("main.c").unwrap(), Path::new("/work/main.c"));
        assert_eq!(resolve("src/a.c").unwrap(), Path::new("/work/src/a.c"));
    }

    #[test]
    fn rejects_escapes() {
        for bad in ["", "/etc/passwd", "../x", "a/../../x", "./main.c"] {
            assert!(resolve(bad).is_err(), "{bad:?}");
        }
    }
}

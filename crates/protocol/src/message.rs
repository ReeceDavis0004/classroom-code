use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToRunner {
    PutFile {
        path: String,
        bytes: Vec<u8>,
    },
    Exec {
        argv: Vec<String>,
        stdin: Vec<u8>,
        cpu_limit_ms: u32,
        wall_limit_ms: u32,
        stdout_cap: u32,
        stderr_cap: u32,
    },
    Stat {
        path: String,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FromRunner {
    Ready {
        runner_version: u32,
    },
    FileOk,
    Stat {
        exists: bool,
        size: u64,
    },
    Exited {
        exit_code: Option<i32>,
        signal: Option<i32>,
        cpu_ms: u32,
        wall_ms: u32,
        max_rss_kb: u32,
        oom_kills: u32,
        timed_out: bool,
        stdout: Vec<u8>,
        stdout_truncated: bool,
        stderr: Vec<u8>,
        stderr_truncated: bool,
    },
    Error {
        message: String,
    },
}

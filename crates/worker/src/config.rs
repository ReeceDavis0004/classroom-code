use bollard::models::{
    ContainerCreateBody, HostConfig, HostConfigCgroupnsModeEnum, HostConfigLogConfig,
    ResourcesUlimits,
};
use std::collections::HashMap;

const SANDBOX_UID: &str = "10001";
const MIB: i64 = 1 << 20;

pub struct SandboxSpec {
    pub image: String,
    pub worker_id: String,
    pub job_id: String,
    pub attempt: u32,
    pub memory_mb: i64,
    pub pids_limit: i64,
}

impl SandboxSpec {
    pub fn name(&self) -> String {
        format!("judge-{}-{}", self.job_id, self.attempt)
    }

    pub fn container_body(&self) -> ContainerCreateBody {
        let memory = self.memory_mb * MIB;
        let host_config = HostConfig {
            network_mode: Some("none".into()),
            readonly_rootfs: Some(true),
            tmpfs: Some(HashMap::from([(
                "/work".into(),
                format!("rw,exec,nosuid,nodev,size=64m,mode=0700,uid={SANDBOX_UID}"),
            )])),
            ipc_mode: Some("none".into()),
            cgroupns_mode: Some(HostConfigCgroupnsModeEnum::PRIVATE),
            cap_drop: Some(vec!["ALL".into()]),
            security_opt: Some(vec!["no-new-privileges".into()]),
            memory: Some(memory),
            memory_swap: Some(memory),
            nano_cpus: Some(1_000_000_000),
            pids_limit: Some(self.pids_limit),
            ulimits: Some(vec![
                ulimit("nofile", 64),
                ulimit("core", 0),
                ulimit("fsize", 16 * MIB),
            ]),
            log_config: Some(HostConfigLogConfig {
                typ: Some("none".into()),
                config: None,
            }),
            init: Some(false),
            auto_remove: Some(false),
            ..Default::default()
        };
        ContainerCreateBody {
            image: Some(self.image.clone()),
            entrypoint: Some(vec!["/judge/runner".into()]),
            user: Some(format!("{SANDBOX_UID}:{SANDBOX_UID}")),
            env: Some(vec![
                "PATH=/usr/local/bin:/usr/bin:/bin".into(),
                "HOME=/work".into(),
                "TMPDIR=/work/tmp".into(),
                "LANG=C.UTF-8".into(),
            ]),
            open_stdin: Some(true),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            labels: Some(HashMap::from([
                ("judge.worker".into(), self.worker_id.clone()),
                ("judge.job".into(), self.job_id.clone()),
                ("judge.attempt".into(), self.attempt.to_string()),
            ])),
            host_config: Some(host_config),
            ..Default::default()
        }
    }
}

fn ulimit(name: &str, value: i64) -> ResourcesUlimits {
    ResourcesUlimits {
        name: Some(name.into()),
        soft: Some(value),
        hard: Some(value),
    }
}

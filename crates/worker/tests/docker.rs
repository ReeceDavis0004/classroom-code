use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::models::HostConfigLogConfig;
use bollard::query_parameters::{CreateContainerOptionsBuilder, RemoveContainerOptionsBuilder};
use futures_util::StreamExt;
use protocol::{FromRunner, ToRunner};
use std::time::Duration;
use worker::{Sandbox, SandboxSpec};

const IMAGE: &str = "judge-spike:dev";
const MIB: i64 = 1 << 20;
const STEP: Duration = Duration::from_secs(5);

async fn docker() -> Docker {
    let docker = Docker::connect_with_local_defaults().unwrap();
    docker.inspect_image(IMAGE).await.unwrap_or_else(|e| {
        panic!("{IMAGE} missing ({e}); run: docker build -f images/spike/Dockerfile -t {IMAGE} .")
    });
    docker
}

fn spec(job_id: &str) -> SandboxSpec {
    SandboxSpec {
        image: IMAGE.into(),
        worker_id: "test".into(),
        job_id: job_id.into(),
        attempt: 1,
        memory_mb: 512,
        pids_limit: 32,
    }
}

async fn assert_gone(docker: &Docker, id: &str) {
    match docker.inspect_container(id, None).await {
        Err(DockerError::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        other => panic!("container {id} still present: {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs Docker and the judge-spike:dev image"]
async fn full_cycle_with_section_5_config() {
    let docker = docker().await;
    let mut sandbox = Sandbox::start(&docker, &spec("it-cycle")).await.unwrap();
    let id = sandbox.id.clone();

    assert_eq!(
        sandbox.recv(STEP).await.unwrap(),
        FromRunner::Ready { runner_version: 1 }
    );

    let source = b"int main(void) { return 0; }\n".to_vec();
    sandbox
        .send(&ToRunner::PutFile {
            path: "main.c".into(),
            bytes: source.clone(),
        })
        .await
        .unwrap();
    assert_eq!(sandbox.recv(STEP).await.unwrap(), FromRunner::FileOk);

    sandbox
        .send(&ToRunner::PutFile {
            path: "../x".into(),
            bytes: vec![1],
        })
        .await
        .unwrap();
    assert!(matches!(
        sandbox.recv(STEP).await.unwrap(),
        FromRunner::Error { .. }
    ));

    sandbox
        .send(&ToRunner::Stat {
            path: "main.c".into(),
        })
        .await
        .unwrap();
    assert_eq!(
        sandbox.recv(STEP).await.unwrap(),
        FromRunner::Stat {
            exists: true,
            size: source.len() as u64
        }
    );

    let info = docker.inspect_container(&id, None).await.unwrap();
    let config = info.config.unwrap();
    let host = info.host_config.unwrap();
    assert_eq!(config.user.as_deref(), Some("10001:10001"));
    assert_eq!(config.tty, Some(false));
    assert_eq!(config.open_stdin, Some(true));
    assert_eq!(host.readonly_rootfs, Some(true));
    assert_eq!(host.network_mode.as_deref(), Some("none"));
    assert_eq!(host.ipc_mode.as_deref(), Some("none"));
    assert_eq!(host.cap_drop, Some(vec!["ALL".to_string()]));
    assert_eq!(host.cap_add, None);
    assert_eq!(
        host.security_opt,
        Some(vec!["no-new-privileges".to_string()])
    );
    assert_eq!(host.memory, Some(512 * MIB));
    assert_eq!(host.memory_swap, Some(512 * MIB));
    assert_eq!(host.nano_cpus, Some(1_000_000_000));
    assert_eq!(host.pids_limit, Some(32));
    assert_eq!(host.ulimits.map(|u| u.len()), Some(3));
    assert_eq!(
        host.log_config,
        Some(HostConfigLogConfig {
            typ: Some("none".into()),
            config: Some(Default::default()),
        })
    );
    assert_eq!(host.binds, None);
    assert_eq!(info.mounts.map(|m| m.len()), Some(0));

    sandbox.send(&ToRunner::Shutdown).await.unwrap();
    let exit = docker.wait_container(&id, None).next().await.unwrap();
    assert_eq!(exit.unwrap().status_code, 0);

    sandbox.destroy().await.unwrap();
    assert_gone(&docker, &id).await;
}

#[tokio::test]
#[ignore = "needs Docker and the judge-spike:dev image"]
async fn stale_container_with_same_name_is_replaced() {
    let docker = docker().await;
    let spec = spec("it-stale");
    let force = RemoveContainerOptionsBuilder::default().force(true).build();
    let _ = docker.remove_container(&spec.name(), Some(force)).await;
    let stale = docker
        .create_container(
            Some(
                CreateContainerOptionsBuilder::default()
                    .name(&spec.name())
                    .build(),
            ),
            spec.container_body(),
        )
        .await
        .unwrap();

    let mut sandbox = Sandbox::start(&docker, &spec).await.unwrap();
    assert_ne!(sandbox.id, stale.id);
    assert_gone(&docker, &stale.id).await;
    assert_eq!(
        sandbox.recv(STEP).await.unwrap(),
        FromRunner::Ready { runner_version: 1 }
    );

    let id = sandbox.id.clone();
    sandbox.destroy().await.unwrap();
    assert_gone(&docker, &id).await;
}

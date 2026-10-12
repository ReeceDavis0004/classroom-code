use bollard::Docker;
use protocol::ToRunner;
use std::time::Duration;
use worker::{Error, Sandbox, SandboxSpec};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let docker = Docker::connect_with_local_defaults()?;
    let spec = SandboxSpec {
        image: "judge-spike:dev".into(),
        worker_id: "spike".into(),
        job_id: "spike".into(),
        attempt: 0,
        memory_mb: 512,
        pids_limit: 32,
    };
    let mut sandbox = Sandbox::start(&docker, &spec).await?;
    let ready = sandbox.recv(Duration::from_secs(5)).await;
    let _ = sandbox.send(&ToRunner::Shutdown).await;
    sandbox.destroy().await?;
    println!("{:?}", ready?);
    Ok(())
}

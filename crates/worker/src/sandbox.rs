use crate::SandboxSpec;
use bollard::Docker;
use bollard::container::{AttachContainerResults, LogOutput};
use bollard::errors::Error as DockerError;
use bollard::query_parameters::{
    AttachContainerOptionsBuilder, CreateContainerOptionsBuilder, RemoveContainerOptionsBuilder,
};
use futures_util::{Stream, StreamExt};
use protocol::{FromRunner, ToRunner, decode, encode};
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct Sandbox {
    docker: Docker,
    pub id: String,
    input: Pin<Box<dyn AsyncWrite + Send>>,
    output: Pin<Box<dyn Stream<Item = Result<LogOutput, DockerError>> + Send>>,
    buf: Vec<u8>,
}

impl Sandbox {
    pub async fn start(docker: &Docker, spec: &SandboxSpec) -> Result<Self, Error> {
        let id = create(docker, spec).await?;
        match attach_and_start(docker, &id).await {
            Ok(attach) => Ok(Self {
                docker: docker.clone(),
                id,
                input: attach.input,
                output: attach.output,
                buf: Vec::new(),
            }),
            Err(e) => {
                let _ = remove(docker, &id).await;
                Err(e)
            }
        }
    }

    pub async fn send(&mut self, msg: &ToRunner) -> Result<(), Error> {
        self.input.write_all(&encode(msg)?).await?;
        self.input.flush().await?;
        Ok(())
    }

    pub async fn recv(&mut self, within: Duration) -> Result<FromRunner, Error> {
        tokio::time::timeout(within, self.next_frame()).await?
    }

    async fn next_frame(&mut self) -> Result<FromRunner, Error> {
        loop {
            if let Some((msg, used)) = decode(&self.buf)? {
                self.buf.drain(..used);
                return Ok(msg);
            }
            match self.output.next().await {
                Some(Ok(LogOutput::StdOut { message })) => self.buf.extend_from_slice(&message),
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
                None => return Err("runner closed stdout".into()),
            }
        }
    }

    pub async fn destroy(self) -> Result<(), Error> {
        remove(&self.docker, &self.id).await
    }
}

async fn attach_and_start(docker: &Docker, id: &str) -> Result<AttachContainerResults, Error> {
    let options = AttachContainerOptionsBuilder::default()
        .stream(true)
        .stdin(true)
        .stdout(true)
        .stderr(true)
        .build();
    let attach = docker.attach_container(id, Some(options)).await?;
    docker.start_container(id, None).await?;
    Ok(attach)
}

async fn create(docker: &Docker, spec: &SandboxSpec) -> Result<String, Error> {
    let name = spec.name();
    let options = CreateContainerOptionsBuilder::default().name(&name).build();
    match docker
        .create_container(Some(options.clone()), spec.container_body())
        .await
    {
        Ok(r) => Ok(r.id),
        Err(DockerError::DockerResponseServerError {
            status_code: 409, ..
        }) => {
            remove(docker, &name).await?;
            Ok(docker
                .create_container(Some(options), spec.container_body())
                .await?
                .id)
        }
        Err(e) => Err(e.into()),
    }
}

async fn remove(docker: &Docker, id: &str) -> Result<(), Error> {
    let options = RemoveContainerOptionsBuilder::default().force(true).build();
    docker.remove_container(id, Some(options)).await?;
    Ok(())
}

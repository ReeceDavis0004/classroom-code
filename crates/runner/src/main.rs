mod files;

use protocol::{FromRunner, ToRunner, decode, encode};
use std::error::Error;
use std::io::{self, Read, Write};
use std::process::ExitCode;

const RUNNER_VERSION: u32 = 1;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("runner: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    send(
        &mut stdout,
        &FromRunner::Ready {
            runner_version: RUNNER_VERSION,
        },
    )?;
    let mut buf = Vec::new();
    let mut chunk = vec![0; 64 * 1024];
    loop {
        while let Some((msg, used)) = decode::<ToRunner>(&buf)? {
            buf.drain(..used);
            let reply = match msg {
                ToRunner::Shutdown => return Ok(()),
                ToRunner::PutFile { path, bytes } => files::put(&path, &bytes),
                ToRunner::Stat { path } => files::stat(&path),
                ToRunner::Exec { .. } => FromRunner::Error {
                    message: "not implemented".into(),
                },
            };
            send(&mut stdout, &reply)?;
        }
        let n = stdin.read(&mut chunk)?;
        if n == 0 {
            return Err("stdin closed before Shutdown".into());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn send(out: &mut impl Write, msg: &FromRunner) -> Result<(), Box<dyn Error>> {
    out.write_all(&encode(msg)?)?;
    out.flush()?;
    Ok(())
}

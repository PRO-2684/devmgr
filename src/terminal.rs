use bollard::{
    Docker,
    container::LogOutput,
    errors::Error,
    query_parameters::{ResizeExecOptions, ResizeExecOptionsBuilder},
};
use futures_util::{Stream, StreamExt};
use std::{
    io::{Read, Write, stdout},
    pin::Pin,
};
use termion::{async_stdin, raw::IntoRawMode, terminal_size};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    spawn,
    task::JoinHandle,
    time::{Duration, sleep},
};

type ExecOutput = Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>;
type ExecInput = Pin<Box<dyn AsyncWrite + Send>>;

pub fn spawn_stdin_pipe(mut input: ExecInput) -> JoinHandle<()> {
    spawn(async move {
        #[allow(clippy::unbuffered_bytes)]
        let mut stdin = async_stdin().bytes();
        loop {
            if let Some(Ok(byte)) = stdin.next() {
                input.write_all(&[byte]).await.ok();
            } else {
                sleep(Duration::from_nanos(10)).await;
            }
        }
    })
}

pub async fn forward_output_to_stdout(mut output: ExecOutput) -> Result<(), Error> {
    let mut stdout = stdout().into_raw_mode()?;

    while let Some(output) = output.next().await {
        match output {
            Ok(output) => {
                stdout.write_all(output.into_bytes().as_ref())?;
                stdout.flush()?;
            }
            Err(err) => eprintln!("Failed to read docker exec output: {err}"),
        }
    }

    Ok(())
}

fn resize_options((width, height): (u16, u16)) -> ResizeExecOptions {
    ResizeExecOptionsBuilder::default()
        .h(i32::from(height))
        .w(i32::from(width))
        .build()
}

pub async fn resize_exec_to_terminal(docker: &Docker, exec_id: &str) -> Result<(), Error> {
    docker
        .resize_exec(exec_id, resize_options(terminal_size()?))
        .await
}

pub fn spawn_terminal_resize_handler(docker: Docker, exec_id: String) -> JoinHandle<()> {
    use tokio::signal::unix::{SignalKind, signal};

    spawn(async move {
        let Ok(mut sigwinch) = signal(SignalKind::window_change()) else {
            return;
        };

        while sigwinch.recv().await.is_some() {
            if let Err(err) = resize_exec_to_terminal(&docker, &exec_id).await {
                if is_stopped_exec_resize_error(&err) {
                    break;
                }
                eprintln!("Failed to resize docker exec tty: {err}");
            }
        }
    })
}

pub fn is_stopped_exec_resize_error(error: &Error) -> bool {
    matches!(
        error,
        Error::DockerResponseServerError {
            status_code: 500,
            message,
        } if message.contains("cannot resize a stopped container")
    )
}

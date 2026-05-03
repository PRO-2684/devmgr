#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

use bollard::{
    Docker,
    errors::Error,
    exec::{CreateExecOptions, StartExecResults},
    models::ContainerSummary,
    query_parameters::{ListContainersOptionsBuilder, ResizeExecOptionsBuilder},
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt,
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Write, stdout},
    path::PathBuf,
};
use termion::{async_stdin, raw::IntoRawMode, terminal_size};
use tokio::{
    io::AsyncWriteExt,
    spawn,
    time::{Duration, sleep},
};

/// A devcontainer.
#[derive(Debug, Clone)]
pub struct Devcontainer<'a> {
    /// The id of the devcontainer.
    pub id: String,
    /// The name of the devcontainer.
    pub name: String,
    /// The path to the devcontainer. (host)
    pub path: PathBuf,
    /// The workspace folder of the devcontainer. (container)
    pub workspace: String,
    /// The user to use in the devcontainer.
    pub user: String,
    /// The docker client.
    pub docker: &'a Docker,
}

impl<'a> Devcontainer<'a> {
    // Creation

    /// Iterate over all running devcontainers on the machine.
    pub async fn iter(docker: &'a Docker) -> Result<impl Iterator<Item = Self> + 'a, Error> {
        let filters = HashMap::from([("label", vec!["devcontainer.local_folder"])]);
        let option = ListContainersOptionsBuilder::default()
            .filters(&filters)
            .build();
        let containers = docker.list_containers(Some(option)).await?;

        let it = containers
            .into_iter()
            .filter_map(|container| Self::from_container_summary(docker, container));

        Ok(it)
    }

    /// Try to find a devcontainer at the given path.
    pub async fn from_path(docker: &'a Docker, path: &PathBuf) -> Result<Option<Self>, Error> {
        let path = path.canonicalize()?;
        let path_str = path.to_str().ok_or_else(|| Error::IOError {
            err: IoError::new(
                IoErrorKind::InvalidData,
                "Path contains invalid UTF-8 characters",
            ),
        })?;
        let filters = HashMap::from([(
            "label",
            vec![format!("devcontainer.local_folder={path_str}")],
        )]);
        let option = ListContainersOptionsBuilder::default()
            .filters(&filters)
            .build();
        let containers = docker.list_containers(Some(option)).await?;

        for container in containers {
            if let Some(devcontainer) = Self::from_container_summary(docker, container) {
                if devcontainer.path == path {
                    return Ok(Some(devcontainer));
                }
            }
        }

        Ok(None)
    }

    /// Create a [`Devcontainer`] from given [`ContainerSummary`].
    pub fn from_container_summary(docker: &'a Docker, container: ContainerSummary) -> Option<Self> {
        let id = container.id?;
        let name = container.names?.get(0)?.trim_start_matches('/').to_string();

        let labels = container.labels?;
        let path = labels.get("devcontainer.local_folder")?;
        let path = PathBuf::from(path);

        // Iterate over mounts to find the workspace folder
        let workspace = container.mounts?.into_iter().find_map(|mount| {
            // Source is the local folder, destination is the workspace folder
            if let Some(src) = &mount.source
                && src == path.to_str()?
            {
                return mount.destination;
            }
            None
        })?;

        let metadata = labels.get("devcontainer.metadata")?;
        let metadata: DevcontainerMetadata = serde_json::from_str(metadata).ok()?;
        let user = metadata.remote_user;

        Some(Self {
            id,
            name,
            path,
            workspace,
            user,
            docker,
        })
    }

    // Actions

    /// Attach to the devcontainer using `docker exec`.
    // https://github.com/fussybeaver/bollard/blob/94f4e5388a5fc7dd69db4d8d39cc8e6fa1937760/examples/exec_term.rs
    pub async fn attach(&self) -> Result<(), Error> {
        self.exec(vec!["bash"]).await
    }

    /// Execute a command in the devcontainer using `docker exec`.
    pub async fn exec(&self, cmd: Vec<&str>) -> Result<(), Error> {
        let option = CreateExecOptions {
            cmd: Some(cmd),
            attach_stderr: Some(true),
            attach_stdout: Some(true),
            attach_stdin: Some(true),
            tty: Some(true),
            user: Some(&self.user),
            working_dir: Some(&self.workspace),
            // detach_keys: None,
            // env: None,
            // privileged: Some(false),
            ..Default::default()
        };
        let exec_id = self.docker.create_exec(&self.id, option).await?.id;
        let StartExecResults::Attached {
            mut output,
            mut input,
        } = self.docker.start_exec(&exec_id, None).await?
        else {
            // TODO: Error?
            return Ok(());
        };

        // pipe stdin into the docker exec stream input
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
        });

        // resize the docker exec tty to match the terminal size
        let tty_size = terminal_size()?;
        self.docker
            .resize_exec(
                &exec_id,
                ResizeExecOptionsBuilder::default()
                    .h(tty_size.1 as i32)
                    .w(tty_size.0 as i32)
                    .build(),
            )
            .await?;

        // set stdout in raw mode so we can do tty stuff
        let stdout = stdout();
        let mut stdout = stdout.lock().into_raw_mode()?;

        // pipe docker exec output into stdout
        while let Some(Ok(output)) = output.next().await {
            stdout.write_all(output.into_bytes().as_ref())?;
            stdout.flush()?;
        }

        Ok(())
    }
}

impl<'a> fmt::Display for Devcontainer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // Detailed display with newlines and indentation
            write!(
                f,
                "Devcontainer {}\n  Name: {}\n  Path: {}\n  Workspace: {}\n  User: {}",
                self.id,
                self.name,
                self.path.display(),
                self.workspace,
                self.user
            )
        } else {
            // Compact display
            write!(f, "Devcontainer {} ({})", self.id, self.path.display())
        }
    }
}

/// Metadata (that we're interested in) for a devcontainer.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DevcontainerMetadata {
    /// The user to use in the devcontainer.
    pub remote_user: String,
}

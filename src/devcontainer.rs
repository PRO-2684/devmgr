use bollard::{
    Docker,
    errors::Error,
    models::{ContainerSummary, ContainerSummaryStateEnum},
    query_parameters::ListContainersOptionsBuilder,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::{Path, PathBuf},
};

/// A devcontainer.
#[derive(Debug, Clone)]
pub struct Devcontainer<'a> {
    /// The id of the devcontainer.
    pub id: String,
    /// The name of the devcontainer.
    pub name: String,
    /// If the devcontainer is running.
    pub running: bool,
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
    /// Iterate over all devcontainers on the machine.
    ///
    /// # Errors
    ///
    /// Returns an error if the docker client fails to list containers.
    pub async fn iter(
        docker: &'a Docker,
        all: bool,
    ) -> Result<impl Iterator<Item = Self> + 'a, Error> {
        let filters = HashMap::from([("label", vec!["devcontainer.local_folder"])]);
        let option = ListContainersOptionsBuilder::default()
            .all(all)
            .filters(&filters)
            .build();
        let containers = docker.list_containers(Some(option)).await?;

        let it = containers
            .into_iter()
            .filter_map(|container| Self::from_container_summary(docker, container));

        Ok(it)
    }

    /// Try to find a devcontainer at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the docker client fails to list containers or if the path is invalid.
    pub async fn from_path(docker: &'a Docker, path: &Path) -> Result<Option<Self>, Error> {
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
            if let Some(devcontainer) = Self::from_container_summary(docker, container)
                && devcontainer.path == path
            {
                return Ok(Some(devcontainer));
            }
        }

        Ok(None)
    }

    /// Create a [`Devcontainer`] from given [`ContainerSummary`].
    #[must_use]
    pub fn from_container_summary(docker: &'a Docker, container: ContainerSummary) -> Option<Self> {
        let id = container.id?;
        let name = container
            .names?
            .first()?
            .trim_start_matches('/')
            .to_string();

        let labels = container.labels?;
        let path = labels.get("devcontainer.local_folder")?;
        let path = PathBuf::from(path);

        let workspace = container.mounts?.into_iter().find_map(|mount| {
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
            running: container.state == Some(ContainerSummaryStateEnum::RUNNING),
            path,
            workspace,
            user,
            docker,
        })
    }
}

impl fmt::Display for Devcontainer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{} Devcontainer {}\n  Name: {}\n  Path: {}\n  Workspace: {}\n  User: {}",
                if self.running { "🟢" } else { "🔴" },
                self.id,
                self.name,
                self.path.display(),
                self.workspace,
                self.user
            )
        } else {
            write!(
                f,
                "{} Devcontainer {} ({})",
                if self.running { "🟢" } else { "🔴" },
                self.id,
                self.path.display()
            )
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

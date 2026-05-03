#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

use bollard::{
    Docker,
    errors::Error,
    models::ContainerSummary,
    query_parameters::{ListContainersOptions, ListContainersOptionsBuilder},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::PathBuf,
};

/// A devcontainer.
#[derive(Debug, Clone)]
pub struct Devcontainer {
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
}

impl Devcontainer {
    /// Iterate over all running devcontainers on the machine.
    pub async fn iter(docker: &Docker) -> Result<impl Iterator<Item = Self> + '_, Error> {
        let filters = HashMap::from([("label", vec!["devcontainer.local_folder"])]);
        let option = ListContainersOptionsBuilder::default()
            .filters(&filters)
            .build();
        let containers = docker.list_containers(Some(option)).await?;

        let it = containers
            .into_iter()
            .filter_map(Self::from_container_summary);

        Ok(it)
    }

    /// Try to find a devcontainer at the given path.
    pub async fn from_path(docker: &Docker, path: &PathBuf) -> Result<Option<Self>, Error> {
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
            if let Some(devcontainer) = Self::from_container_summary(container) {
                if devcontainer.path == path {
                    return Ok(Some(devcontainer));
                }
            }
        }

        Ok(None)
    }

    /// Create a [`Devcontainer`] from given [`ContainerSummary`].
    pub fn from_container_summary(container: ContainerSummary) -> Option<Self> {
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
        })
    }
}

impl fmt::Display for Devcontainer {
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
            write!(
                f,
                "Devcontainer {} (Name: {}, Path: {}, Workspace: {}, User: {})",
                self.id,
                self.name,
                self.path.display(),
                self.workspace,
                self.user
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

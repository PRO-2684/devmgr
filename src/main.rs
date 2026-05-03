#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

use bollard::{Docker, errors::Error};
use devctl::Devcontainer;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let docker = Docker::connect_with_local_defaults()?;
    let devcontainers = Devcontainer::iter(&docker).await?;

    for devcontainer in devcontainers {
        println!("{devcontainer:#}");
    }

    Ok(())
}

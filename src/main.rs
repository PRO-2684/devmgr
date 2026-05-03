#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

use std::{
    io::{Error as IoError, ErrorKind},
    path::PathBuf,
};

use argh::FromArgs;
use bollard::{Docker, errors::Error};
use devmgr::Devcontainer;

/// Small utility for managing devcontainers.
#[derive(FromArgs)]
#[argh(help_triggers("-h", "--help"))]
struct Args {
    #[argh(subcommand)]
    command: Command,
    /// show verbose output.
    #[argh(switch, short = 'v')]
    verbose: bool,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    List(ListArgs),
    Exec(ExecArgs),
    Attach(AttachArgs),
}

/// List all devcontainers.
#[derive(FromArgs)]
#[argh(subcommand, name = "ls")]
struct ListArgs {}

/// Execute a command in a devcontainer.
#[derive(FromArgs)]
#[argh(subcommand, name = "exec")]
struct ExecArgs {
    /// path to the devcontainer local folder.
    #[argh(option, short = 'p', default = "PathBuf::from(\".\")")]
    path: PathBuf,
    /// the command to execute.
    #[argh(positional)]
    command: Vec<String>,
}

/// Attach to a devcontainer.
#[derive(FromArgs)]
#[argh(subcommand, name = "att")]
struct AttachArgs {
    /// path to the devcontainer local folder.
    #[argh(option, short = 'p', default = "PathBuf::from(\".\")")]
    path: PathBuf,
}

async fn from_path_or_error<'a>(
    docker: &'a Docker,
    path: &'a PathBuf,
) -> Result<Devcontainer<'a>, Error> {
    Devcontainer::from_path(docker, path)
        .await?
        .ok_or_else(|| Error::IOError {
            err: IoError::new(
                ErrorKind::NotFound,
                "devcontainer not found in the specified path",
            ),
        })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();
    let docker = Docker::connect_with_local_defaults()?;
    match args.command {
        Command::List(_) => {
            let devcontainers = Devcontainer::iter(&docker).await?;

            if args.verbose {
                for devcontainer in devcontainers {
                    println!("{devcontainer:#}");
                }
            } else {
                for devcontainer in devcontainers {
                    println!("{devcontainer}");
                }
            }
        }
        Command::Exec(exec_args) => {
            let devcontainer = from_path_or_error(&docker, &exec_args.path).await?;
            if args.verbose {
                eprintln!("Found: {devcontainer:#}");
            }
            let command: Vec<&str> = exec_args.command.iter().map(String::as_str).collect();
            devcontainer.exec(command).await?;
            if args.verbose {
                eprintln!("Exited devcontainer at {}", devcontainer.path.display());
            }
        }
        Command::Attach(attach_args) => {
            let devcontainer = from_path_or_error(&docker, &attach_args.path).await?;
            if args.verbose {
                eprintln!("Found: {devcontainer:#}");
            }
            devcontainer.attach().await?;
            if args.verbose {
                eprintln!("Exited devcontainer at {}", devcontainer.path.display());
            }
        }
    }

    Ok(())
}

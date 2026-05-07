#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

use std::{
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
    process::ExitCode,
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
    Info(InfoArgs),
    Exec(ExecArgs),
    Attach(AttachArgs),
}

/// List all devcontainers.
#[derive(FromArgs)]
#[argh(subcommand, name = "list", short = 'l', help_triggers("-h", "--help"))]
struct ListArgs {
    /// show all devcontainers, including those that are not running.
    #[argh(switch, short = 'a')]
    all: bool,
}

/// Show detailed information about a devcontainer.
#[derive(FromArgs)]
#[argh(subcommand, name = "info", short = 'i', help_triggers("-h", "--help"))]
struct InfoArgs {
    /// path to the devcontainer local folder.
    #[argh(option, short = 'p', default = "PathBuf::from(\".\")")]
    path: PathBuf,
}

/// Execute a command in a devcontainer.
#[derive(FromArgs)]
#[argh(subcommand, name = "exec", short = 'e', help_triggers("-h", "--help"))]
struct ExecArgs {
    /// path to the devcontainer local folder.
    #[argh(option, short = 'p', default = "PathBuf::from(\".\")")]
    path: PathBuf,
    /// the SHELL environment variable passed to the devcontainer. defaults to /bin/bash.
    #[argh(option, short = 's', default = "String::from(\"/bin/bash\")")]
    shell: String,
    /// the command to execute.
    #[argh(positional)]
    command: Vec<String>,
}

/// Attach to a devcontainer.
#[derive(FromArgs)]
#[argh(subcommand, name = "att", short = 'a', help_triggers("-h", "--help"))]
struct AttachArgs {
    /// path to the devcontainer local folder.
    #[argh(option, short = 'p', default = "PathBuf::from(\".\")")]
    path: PathBuf,
    /// the shell to use when attaching. defaults to /bin/bash.
    #[argh(option, short = 's', default = "String::from(\"/bin/bash\")")]
    shell: String,
}

async fn from_path_or_error<'a>(
    docker: &'a Docker,
    path: &'a Path,
) -> Result<Devcontainer<'a>, Error> {
    Devcontainer::from_path(docker, path).await?.ok_or_else(|| {
        io_error(format!(
            "Devcontainer not found in the specified path: {}",
            path.display()
        ))
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Args = argh::from_env();
    match run(&args).await {
        Ok(Some(0) | None) => ExitCode::SUCCESS,
        Ok(Some(code)) => {
            if args.verbose {
                eprintln!("Exited with code {code}");
            }
            let Ok(code) = u8::try_from(code) else {
                eprintln!("Exit code {code} is out of range for u8");
                return ExitCode::FAILURE;
            };
            ExitCode::from(code)
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: &Args) -> Result<Option<i64>, Error> {
    let docker = Docker::connect_with_local_defaults()?;
    let status = match &args.command {
        Command::List(list_args) => {
            let devcontainers = Devcontainer::iter(&docker, list_args.all).await?;

            if args.verbose {
                for devcontainer in devcontainers {
                    println!("{devcontainer:#}");
                }
            } else {
                for devcontainer in devcontainers {
                    println!("{devcontainer}");
                }
            }
            None
        }
        Command::Info(info_args) => {
            let devcontainer = from_path_or_error(&docker, &info_args.path).await?;
            println!("{devcontainer:#}");
            None
        }
        Command::Exec(exec_args) => {
            let devcontainer = from_path_or_error(&docker, &exec_args.path).await?;
            if args.verbose {
                eprintln!("Found: {devcontainer:#}");
            }
            let command: Vec<&str> = exec_args.command.iter().map(String::as_str).collect();
            let status = devcontainer.exec(command, &exec_args.shell).await?;
            if args.verbose {
                eprintln!("Exited devcontainer at {}", devcontainer.path.display());
            }
            Some(status)
        }
        Command::Attach(attach_args) => {
            let devcontainer = from_path_or_error(&docker, &attach_args.path).await?;
            if args.verbose {
                eprintln!("Found: {devcontainer:#}");
            }
            let status = devcontainer.attach(&attach_args.shell).await?;
            if args.verbose {
                eprintln!("Exited devcontainer at {}", devcontainer.path.display());
            }
            Some(status)
        }
    };

    Ok(status)
}

fn io_error(msg: String) -> Error {
    Error::IOError {
        err: IoError::new(ErrorKind::NotFound, msg),
    }
}

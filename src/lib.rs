#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

use bollard::{
    Docker,
    errors::Error,
    exec::{CreateExecOptions, StartExecResults},
    models::{ContainerSummary, ContainerSummaryStateEnum},
    query_parameters::{ListContainersOptionsBuilder, ResizeExecOptionsBuilder},
};
use crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste, Event, EventStream, KeyCode, KeyEvent,
        KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size},
};
use futures_util::{FutureExt, StreamExt, select};
use serde::Deserialize;
use std::{
    collections::HashMap,
    env::var as env_var,
    fmt,
    io::{Error as IoError, ErrorKind as IoErrorKind, Write, stdout},
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

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
    // Creation

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
            running: container.state == Some(ContainerSummaryStateEnum::RUNNING),
            path,
            workspace,
            user,
            docker,
        })
    }

    // Actions

    // https://github.com/fussybeaver/bollard/blob/94f4e5388a5fc7dd69db4d8d39cc8e6fa1937760/examples/exec_term.rs
    /// Attach to the devcontainer using `docker exec`.
    ///
    /// # Errors
    ///
    /// Returns an error if the docker client fails to create or start the exec session, or if there is an I/O error while attaching to the session.
    pub async fn attach(&self, shell: &str) -> Result<(), Error> {
        self.exec(vec![shell], shell).await
    }

    /// Execute a command in the devcontainer using `docker exec`.
    ///
    /// # Errors
    ///
    /// Returns an error if the docker client fails to create or start the exec session, or if there is an I/O error while attaching to the session.
    pub async fn exec(&self, cmd: Vec<&str>, shell: &str) -> Result<(), Error> {
        let term = env_var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
        let term = format!("TERM={term}");
        let shell = format!("SHELL={shell}");
        let option = CreateExecOptions {
            cmd: Some(cmd),
            attach_stderr: Some(true),
            attach_stdout: Some(true),
            attach_stdin: Some(true),
            tty: Some(true),
            user: Some(&self.user),
            working_dir: Some(&self.workspace),
            env: Some(vec![&term, &shell]),
            // detach_keys: None,
            // privileged: Some(false),
            ..Default::default()
        };
        let exec_id = self.docker.create_exec(&self.id, option).await?.id;
        let tty_size = terminal_size()?;

        let StartExecResults::Attached {
            mut output,
            mut input,
        } = self.docker.start_exec(&exec_id, None).await?
        else {
            // TODO: Error?
            return Ok(());
        };

        // Resize is best-effort: short-lived commands can exit before Docker accepts the resize.
        if let Err(err) = resize_exec_tty(self.docker, &exec_id, tty_size.0, tty_size.1).await
            && !is_stopped_exec_resize_error(&err)
        {
            return Err(err);
        }

        let _raw_mode = RawMode::enable()?;
        let _bracketed_paste = BracketedPaste::enable();
        let mut terminal_events = EventStream::new();
        let mut stdout = stdout();

        loop {
            let mut docker_output = output.next().fuse();
            let mut terminal_event = terminal_events.next().fuse();

            select! {
                docker_output = docker_output => {
                    let Some(output) = docker_output else {
                        break;
                    };
                    let output = output?;
                    stdout.write_all(output.into_bytes().as_ref())?;
                    stdout.flush()?;
                },
                terminal_event = terminal_event => {
                    let Some(event) = terminal_event else {
                        break;
                    };
                    match event? {
                        Event::Key(key) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => {
                            if let Some(bytes) = key_event_bytes(key) {
                                input.write_all(&bytes).await?;
                            }
                        }
                        Event::Paste(text) => {
                            input.write_all(text.as_bytes()).await?;
                        }
                        Event::Resize(columns, rows) => {
                            if let Err(err) = resize_exec_tty(self.docker, &exec_id, columns, rows).await
                                && !is_stopped_exec_resize_error(&err)
                            {
                                return Err(err);
                            }
                        }
                        _ => {}
                    }
                },
            }
        }

        Ok(())
    }
}

struct RawMode;

impl RawMode {
    fn enable() -> std::io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

struct BracketedPaste {
    enabled: bool,
}

impl BracketedPaste {
    fn enable() -> Self {
        let enabled = execute!(stdout(), EnableBracketedPaste).is_ok();
        Self { enabled }
    }
}

impl Drop for BracketedPaste {
    fn drop(&mut self) {
        if self.enabled {
            let _ = execute!(stdout(), DisableBracketedPaste);
        }
    }
}

async fn resize_exec_tty(
    docker: &Docker,
    exec_id: &str,
    columns: u16,
    rows: u16,
) -> Result<(), Error> {
    docker
        .resize_exec(
            exec_id,
            ResizeExecOptionsBuilder::default()
                .h(i32::from(rows))
                .w(i32::from(columns))
                .build(),
        )
        .await
}

fn key_event_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    TerminalInput::from_key_event(key).map(TerminalInput::into_bytes)
}

enum TerminalInput {
    Bytes(Vec<u8>),
    Csi { parameters: String, final_byte: u8 },
    Ss3(u8),
}

// Crossterm normalizes platform-specific keyboard input into KeyEvent values,
// but Docker's TTY stream still expects terminal input bytes for the container.
// Keep that translation small and local to this adapter.
impl TerminalInput {
    fn from_key_event(key: KeyEvent) -> Option<Self> {
        let modifiers = key.modifiers;

        special_key_input(key.code, modifiers).or_else(|| character_input(key.code, modifiers))
    }

    fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Csi {
                parameters,
                final_byte,
            } => csi_sequence(&parameters, final_byte),
            Self::Ss3(final_byte) => vec![ESC, b'O', final_byte],
        }
    }
}

const ESC: u8 = 0x1b;
const DEL: u8 = 0x7f;

fn character_input(code: KeyCode, modifiers: KeyModifiers) -> Option<TerminalInput> {
    let mut bytes = Vec::new();

    if modifiers.contains(KeyModifiers::ALT) {
        bytes.push(ESC);
    }

    match code {
        KeyCode::Backspace => bytes.push(DEL),
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Tab => bytes.push(b'\t'),
        KeyCode::Char(char) if modifiers.contains(KeyModifiers::CONTROL) => {
            bytes.push(control_char_byte(char)?);
        }
        KeyCode::Char(char) => {
            let mut buf = [0; 4];
            bytes.extend_from_slice(char.encode_utf8(&mut buf).as_bytes());
        }
        KeyCode::Esc => bytes.push(ESC),
        KeyCode::Null
        | KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Up
        | KeyCode::Down
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::PageUp
        | KeyCode::PageDown
        | KeyCode::BackTab
        | KeyCode::Delete
        | KeyCode::Insert
        | KeyCode::F(_)
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => return None,
    }

    Some(TerminalInput::Bytes(bytes))
}

fn special_key_input(code: KeyCode, modifiers: KeyModifiers) -> Option<TerminalInput> {
    let modifier_number = key_modifier_number(modifiers);

    if let Some(final_byte) = match code {
        KeyCode::Left => Some(b'D'),
        KeyCode::Right => Some(b'C'),
        KeyCode::Up => Some(b'A'),
        KeyCode::Down => Some(b'B'),
        KeyCode::Home => Some(b'H'),
        KeyCode::End => Some(b'F'),
        _ => None,
    } {
        return Some(TerminalInput::Csi {
            parameters: modifier_number.map_or_else(String::new, |modifier_number| {
                format!("1;{modifier_number}")
            }),
            final_byte,
        });
    }

    if code == KeyCode::BackTab {
        return Some(TerminalInput::Csi {
            parameters: String::new(),
            final_byte: b'Z',
        });
    }

    let number = match code {
        KeyCode::Insert => 2,
        KeyCode::Delete => 3,
        KeyCode::PageUp => 5,
        KeyCode::PageDown => 6,
        KeyCode::F(5) => 15,
        KeyCode::F(6) => 17,
        KeyCode::F(7) => 18,
        KeyCode::F(8) => 19,
        KeyCode::F(9) => 20,
        KeyCode::F(10) => 21,
        KeyCode::F(11) => 23,
        KeyCode::F(12) => 24,
        _ => return function_key_input(code),
    };

    Some(TerminalInput::Csi {
        parameters: modifier_number.map_or_else(
            || number.to_string(),
            |modifier_number| format!("{number};{modifier_number}"),
        ),
        final_byte: b'~',
    })
}

fn csi_sequence(parameters: &str, final_byte: u8) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(parameters.len() + 3);
    bytes.extend_from_slice(&[ESC, b'[']);
    bytes.extend_from_slice(parameters.as_bytes());
    bytes.push(final_byte);
    bytes
}

fn key_modifier_number(modifiers: KeyModifiers) -> Option<u8> {
    let mut number = 1;

    if modifiers.contains(KeyModifiers::SHIFT) {
        number += 1;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        number += 2;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        number += 4;
    }

    (number > 1).then_some(number)
}

const fn control_char_byte(char: char) -> Option<u8> {
    match char {
        'a'..='z' => Some(char as u8 - b'a' + 1),
        'A'..='Z' => Some(char as u8 - b'A' + 1),
        '[' | '3' => Some(ESC),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '7' | '/' => Some(0x1f),
        '8' | '?' => Some(DEL),
        ' ' | '2' | '@' => Some(0),
        _ => None,
    }
}

const fn function_key_input(code: KeyCode) -> Option<TerminalInput> {
    match code {
        KeyCode::F(1) => Some(TerminalInput::Ss3(b'P')),
        KeyCode::F(2) => Some(TerminalInput::Ss3(b'Q')),
        KeyCode::F(3) => Some(TerminalInput::Ss3(b'R')),
        KeyCode::F(4) => Some(TerminalInput::Ss3(b'S')),
        _ => None,
    }
}

fn is_stopped_exec_resize_error(error: &Error) -> bool {
    matches!(
        error,
        Error::DockerResponseServerError {
            status_code: 500,
            message,
        } if message.contains("cannot resize a stopped container")
    )
}

impl fmt::Display for Devcontainer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // Detailed display with newlines and indentation
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
            // Compact display
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

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::key_event_bytes;

    #[test]
    fn text_keys_are_forwarded_as_utf8() {
        assert_eq!(
            key_event_bytes(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(vec![b'a'])
        );
        assert_eq!(
            key_event_bytes(KeyEvent::new(KeyCode::Char('é'), KeyModifiers::NONE)),
            Some("é".as_bytes().to_vec())
        );
    }

    #[test]
    fn control_keys_are_forwarded_as_control_bytes() {
        assert_eq!(
            key_event_bytes(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(vec![3])
        );
        assert_eq!(
            key_event_bytes(KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL)),
            Some(vec![0x1b])
        );
    }

    #[test]
    fn navigation_keys_are_forwarded_as_terminal_input_sequences() {
        assert_eq!(
            key_event_bytes(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Some(vec![0x1b, b'[', b'D'])
        );
        assert_eq!(
            key_event_bytes(KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL)),
            Some(b"\x1b[3;5~".to_vec())
        );
    }
}

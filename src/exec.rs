use super::{
    Devcontainer,
    terminal::{
        forward_output_to_stdout, is_stopped_exec_resize_error, resize_exec_to_terminal,
        spawn_stdin_pipe, spawn_terminal_resize_handler,
    },
};
use bollard::{
    errors::Error,
    exec::{CreateExecOptions, StartExecResults},
};
use std::{env::var as env_var, io::Error as IoError};

impl Devcontainer<'_> {
    /// Attach to the devcontainer using `docker exec`. Returns status code.
    ///
    /// # Errors
    ///
    /// Returns an error if the docker client fails to create or start the exec session, or if there is an I/O error while attaching to the session.
    pub async fn attach(&self, shell: &str, shell_args: &[&str]) -> Result<i64, Error> {
        let mut cmd = Vec::with_capacity(shell_args.len() + 1);
        cmd.push(shell);
        cmd.extend_from_slice(shell_args);
        self.exec(cmd, shell).await
    }

    // https://github.com/fussybeaver/bollard/blob/94f4e5388a5fc7dd69db4d8d39cc8e6fa1937760/examples/exec_term.rs
    /// Execute a command in the devcontainer. Returns status code.
    ///
    /// # Errors
    ///
    /// Returns an error if the docker client fails to create or start the exec session, or if there is an I/O error while attaching to the session.
    pub async fn exec(&self, cmd: Vec<&str>, shell: &str) -> Result<i64, Error> {
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
        let resize_handler = spawn_terminal_resize_handler(self.docker.clone(), exec_id.clone());

        let StartExecResults::Attached { output, input } =
            self.docker.start_exec(&exec_id, None).await?
        else {
            return Err(io_error(
                "Failed to start exec session: expected attached result",
            ));
        };

        // Resize is best-effort: short-lived commands can exit before Docker accepts the resize.
        if let Err(err) = resize_exec_to_terminal(self.docker, &exec_id).await
            && !is_stopped_exec_resize_error(&err)
        {
            resize_handler.abort();
            return Err(err);
        }

        let stdin_handle = spawn_stdin_pipe(input);
        let result = forward_output_to_stdout(output).await;
        stdin_handle.abort();
        resize_handler.abort();
        result?;

        // Check status code
        let Some(status) = self.docker.inspect_exec(&exec_id).await?.exit_code else {
            return Err(io_error("Command exit code not found"));
        };
        Ok(status)
    }
}

fn io_error(msg: &str) -> Error {
    Error::IOError {
        err: IoError::other(msg),
    }
}

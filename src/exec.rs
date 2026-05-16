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
        let env = exec_env(shell);
        let env: Vec<&str> = env.iter().map(String::as_str).collect();
        let option = CreateExecOptions {
            cmd: Some(cmd),
            attach_stderr: Some(true),
            attach_stdout: Some(true),
            attach_stdin: Some(true),
            tty: Some(true),
            user: Some(&self.user),
            working_dir: Some(&self.workspace),
            env: Some(env),
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

fn exec_env(shell: &str) -> Vec<String> {
    exec_env_from(
        shell,
        env_var("TERM").ok().as_deref(),
        env_var("LANG").ok().as_deref(),
        env_var("LC_CTYPE").ok().as_deref(),
        env_var("LC_ALL").ok().as_deref(),
    )
}

fn exec_env_from(
    shell: &str,
    term: Option<&str>,
    lang: Option<&str>,
    lc_ctype: Option<&str>,
    lc_all: Option<&str>,
) -> Vec<String> {
    let mut env = vec![
        format!("TERM={}", term.unwrap_or("xterm-256color")),
        format!("SHELL={shell}"),
        format!("LANG={}", utf8_locale_or_default(lang)),
        format!("LC_CTYPE={}", utf8_locale_or_default(lc_ctype)),
    ];

    if let Some(lc_all) = lc_all
        && is_utf8_locale(lc_all)
    {
        env.push(format!("LC_ALL={lc_all}"));
    }

    env
}

fn utf8_locale_or_default(locale: Option<&str>) -> &str {
    locale
        .filter(|locale| is_utf8_locale(locale))
        .unwrap_or("C.UTF-8")
}

fn is_utf8_locale(locale: &str) -> bool {
    let locale = locale.to_ascii_lowercase();
    locale.contains("utf-8") || locale.contains("utf8")
}

#[cfg(test)]
mod tests {
    use super::exec_env_from;

    #[test]
    fn exec_env_defaults_to_utf8_locale() {
        assert_eq!(
            exec_env_from("/bin/bash", None, None, None, None),
            vec![
                "TERM=xterm-256color",
                "SHELL=/bin/bash",
                "LANG=C.UTF-8",
                "LC_CTYPE=C.UTF-8",
            ]
        );
    }

    #[test]
    fn exec_env_preserves_utf8_locale() {
        assert_eq!(
            exec_env_from(
                "/bin/zsh",
                Some("screen-256color"),
                Some("zh_CN.UTF-8"),
                Some("en_US.utf8"),
                Some("C.UTF-8"),
            ),
            vec![
                "TERM=screen-256color",
                "SHELL=/bin/zsh",
                "LANG=zh_CN.UTF-8",
                "LC_CTYPE=en_US.utf8",
                "LC_ALL=C.UTF-8",
            ]
        );
    }

    #[test]
    fn exec_env_replaces_non_utf8_locale() {
        assert_eq!(
            exec_env_from(
                "/bin/sh",
                Some("xterm"),
                Some("C"),
                Some("POSIX"),
                Some("C"),
            ),
            vec![
                "TERM=xterm",
                "SHELL=/bin/sh",
                "LANG=C.UTF-8",
                "LC_CTYPE=C.UTF-8",
            ]
        );
    }
}

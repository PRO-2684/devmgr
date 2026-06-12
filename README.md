# devmgr

[![GitHub License](https://img.shields.io/github/license/PRO-2684/devmgr?logo=opensourceinitiative)](https://github.com/PRO-2684/devmgr/blob/main/LICENSE)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/PRO-2684/devmgr/release.yml?logo=githubactions)](https://github.com/PRO-2684/devmgr/blob/main/.github/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/PRO-2684/devmgr?logo=githubactions)](https://github.com/PRO-2684/devmgr/releases)
[![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/PRO-2684/devmgr/total?logo=github)](https://github.com/PRO-2684/devmgr/releases)
[![Crates.io Version](https://img.shields.io/crates/v/devmgr?logo=rust)](https://crates.io/crates/devmgr)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/devmgr?logo=rust)](https://crates.io/crates/devmgr)
[![docs.rs](https://img.shields.io/docsrs/devmgr?logo=rust)](https://docs.rs/devmgr)

> [!NOTE]
> Currently it does not support Windows, because it relies on [`termion`](https://crates.io/crates/termion).

Small utility for managing devcontainers.

## 🪄 Features

- ⚡ Tiny and fast: Less than 2MB binary size. Even faster than `docker` CLI.
  <details>
  <summary>Benchmark details (commit b11b28c)</summary>

  - Command: `hyperfine --warmup 3 'devmgr exec -- ls -la' 'docker exec -e TERM="${TERM:-xterm-256color}" -u <user> -w <workspace> -t <container> ls -la' --show-output`
  - Result (output truncated):

  <pre><code>Benchmark 1: devmgr exec -- ls -la
    Time (mean ± σ):      48.6 ms ±   3.7 ms    [User: 1.7 ms, System: 1.8 ms]
    Range (min … max):    42.8 ms …  62.8 ms    63 runs

  Benchmark 2: docker exec -e TERM="${TERM:-xterm-256color}" -u vscode -w /workspaces/project1 -t thirsty_pasteur ls -la
    Time (mean ± σ):      74.6 ms ±   4.2 ms    [User: 20.4 ms, System: 23.6 ms]
    Range (min … max):    64.4 ms …  82.8 ms    41 runs
  Summary
    devmgr exec -- ls -la ran
      1.53 ± 0.15 times faster than docker exec -e TERM="${TERM:-xterm-256color}" -u vscode -w /workspaces/project1 -t thirsty_pasteur ls -la
  </code></pre>

  - This is an "unfair" comparison, since `devmgr` have to resolve user, workspace, container name and determine terminal size, but it still outperforms `docker` CLI.
  - We need to `--show_output`, since `devmgr` allocates TTY.

  </details>

- 💡 Smart: Automatically resolves devcontainer based on current (or specified) directory, and retrieves user and workspace configuration.

## 📥 Installation

### Using [`binstall`](https://github.com/cargo-bins/cargo-binstall)

```shell
cargo binstall devmgr
```

### Downloading from Releases

Navigate to the [Releases page](https://github.com/PRO-2684/devmgr/releases) and download respective binary for your platform. Make sure to give it execute permissions.

### Compiling from Source

```shell
cargo install devmgr --features=cli
```

## 💡 Examples

List all devcontainers on the system verbosely:

```bash
$ devmgr -v l # Short for "list"
🟢 Devcontainer ded0cb623184a3068762b34115c776eb480477bd82e5faacf8ef9b772d809018
  Name: busy_moser
  Path: /home/ubuntu/project1
  Workspace: /workspaces/project1
  User: vscode
🟢 Devcontainer 6bacc965b6654a7b16d0391ffac5df5e9b13964489b6e3395255357274f9a9a4
  Name: thirsty_pasteur
  Path: /home/ubuntu/project2
  Workspace: /workspaces/project2
  User: vscode
```

Attach to a devcontainer identified by its path, defaulting to current directory:

```bash
$ devmgr a -p ./project1 # Short for "att"
(project1) vscode@host:/workspaces/project1$ whoami
vscode
(project1) vscode@host:/workspaces/project1$ exit
```

By default, `att` starts the selected shell as a login and interactive shell (`-l -i`).
You can replace those shell arguments after `--`:

```bash
$ devmgr a -s /bin/sh -- -i
```

Execute a command in a devcontainer identified by its path, defaulting to current directory:

```bash
$ devmgr e -p ./project1 ls # Short for "exec"
README.md ...
```

When you need options, you can specify them after `--`.

```bash
$ devmgr e -p ./project1 -- ls -la
total 72
drwxrwxr-x 12 vscode vscode 4096 May  1 05:19 .
drwxr-xr-x  3 root   root   4096 Mar  8 13:37 ..
drwxrwxr-x  2 vscode vscode 4096 Mar  7 10:58 .devcontainer
drwxrwxr-x 10 vscode vscode 4096 May  2 02:34 .git
-rw-r--r--  1 vscode vscode 4598 Apr 21 03:54 .gitignore
-rw-r--r--  1 vscode vscode  105 Dec 19 14:52 .gitmodules
drwxr-xr-x  2 vscode vscode 4096 Mar  7 09:55 .vscode
-rw-r--r--  1 vscode vscode 2304 May  1 05:43 README.md
...
```

For some examples to test out `devmgr`, you can check out the [examples directory](./examples) in the repository.

## 📖 Usage

Due to the limits of `argh`, the order of options and arguments is important.

```bash
$ devmgr -h
Usage: devmgr [-v] <command> [<args>]

Small utility for managing devcontainers.

Options:
  -v, --verbose     show verbose output.
  -h, --help        display usage information

Commands:
  list  l           List all devcontainers.
  info  i           Show detailed information about a devcontainer.
  exec  e           Execute a command in a devcontainer.
  att  a            Attach to a devcontainer.
```

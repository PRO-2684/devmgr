# devmgr

[![GitHub License](https://img.shields.io/github/license/PRO-2684/devmgr?logo=opensourceinitiative)](https://github.com/PRO-2684/devmgr/blob/main/LICENSE)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/PRO-2684/devmgr/release.yml?logo=githubactions)](https://github.com/PRO-2684/devmgr/blob/main/.github/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/PRO-2684/devmgr?logo=githubactions)](https://github.com/PRO-2684/devmgr/releases)
[![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/PRO-2684/devmgr/total?logo=github)](https://github.com/PRO-2684/devmgr/releases)
[![Crates.io Version](https://img.shields.io/crates/v/devmgr?logo=rust)](https://crates.io/crates/devmgr)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/devmgr?logo=rust)](https://crates.io/crates/devmgr)
[![docs.rs](https://img.shields.io/docsrs/devmgr?logo=rust)](https://docs.rs/devmgr)

Small utility for managing devcontainers.

## 🪄 Features

- ⚡ Tiny and fast: Less than 2MB binary size. Even faster than `docker` CLI.
    <details><summary>Benchmark details (commit c858e57)</summary>

    - Command: `hyperfine --warmup 3 'devmgr exec -- ls -la' 'docker exec -e TERM="${TERM:-xterm-256color}" -u <user> -w <workspace> -t <container> ls -la' --show-output`
    - Result (output truncated):
        ```
        Benchmark 1: devmgr exec -- ls -la
          Time (mean ± σ):      46.6 ms ±   3.1 ms    [User: 1.6 ms, System: 1.5 ms]
          Range (min … max):    39.9 ms …  53.7 ms    59 runs


        Benchmark 2: docker exec -e TERM="${TERM:-xterm-256color}" -u vscode -w /workspaces/prior -t thirsty_pasteur ls -la
          Time (mean ± σ):      74.8 ms ±   3.5 ms    [User: 18.6 ms, System: 20.9 ms]
          Range (min … max):    68.6 ms …  82.9 ms    40 runs

        Summary
          devmgr exec -- ls -la ran
            1.61 ± 0.13 times faster than docker exec -e TERM="${TERM:-xterm-256color}" -u vscode -w /workspaces/prior -t thirsty_pasteur ls -la
        ```
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
$ devmgr -v ls
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
$ devmgr att -p ./project1
(project1) vscode@host:/workspaces/project1$ whoami
vscode
(project1) vscode@host:/workspaces/project1$ exit
```

Execute a command in a devcontainer identified by its path, defaulting to current directory:

```bash
$ devmgr exec -p ./project1 ls
README.md ...
```

When you need options, you can specify them after `--`.

```bash
$ devmgr exec -p ./project1 -- ls -la
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
  ls                List all devcontainers.
  exec              Execute a command in a devcontainer.
  att               Attach to a devcontainer.
```

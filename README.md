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
Devcontainer ded0cb623184a3068762b34115c776eb480477bd82e5faacf8ef9b772d809018
  Name: busy_moser
  Path: /home/ubuntu/project1
  Workspace: /workspaces/project1
  User: vscode
Devcontainer 6bacc965b6654a7b16d0391ffac5df5e9b13964489b6e3395255357274f9a9a4
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

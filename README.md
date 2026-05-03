# devctl

[![GitHub License](https://img.shields.io/github/license/PRO-2684/devctl?logo=opensourceinitiative)](https://github.com/PRO-2684/devctl/blob/main/LICENSE)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/PRO-2684/devctl/release.yml?logo=githubactions)](https://github.com/PRO-2684/devctl/blob/main/.github/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/PRO-2684/devctl?logo=githubactions)](https://github.com/PRO-2684/devctl/releases)
[![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/PRO-2684/devctl/total?logo=github)](https://github.com/PRO-2684/devctl/releases)
[![Crates.io Version](https://img.shields.io/crates/v/devctl?logo=rust)](https://crates.io/crates/devctl)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/devctl?logo=rust)](https://crates.io/crates/devctl)
[![docs.rs](https://img.shields.io/docsrs/devctl?logo=rust)](https://docs.rs/devctl)

Small utility for devcontainers.

## ⚙️ Automatic Releases Setup

1. [Create a new GitHub repository](https://github.com/new) with the name `devctl` and push this generated project to it.
2. Enable Actions for the repository, and grant "Read and write permissions" to the workflow [here](https://github.com/PRO-2684/devctl/settings/actions).
3. [Generate an API token on crates.io](https://crates.io/settings/tokens/new), with the following setup:

    - `Name`: `devctl`
    - `Expiration`: `No expiration`
    - `Scopes`: `publish-new`, `publish-update`
    - `Crates`: `devctl`

4. [Add a repository secret](https://github.com/PRO-2684/devctl/settings/secrets/actions/new) named `CARGO_TOKEN` with the generated token as its value.
5. Consider removing this section and updating this README with your own project information.

[Trusted Publishing](https://crates.io/docs/trusted-publishing) is a recent feature added to crates.io. To utilize it, first make sure you've already successfully published the crate to crates.io. Then, follow these steps:

1. [Add a new trusted publisher](https://crates.io/crates/devctl/settings/new-trusted-publisher) to your crate.
    - Set "Workflow filename" to `release.yml`.
    - Keep other fields intact.
    - Click "Add".
2. Modify [`release.yml`](.github/workflows/release.yml).
    1. Comment out or remove the `publish-release` job.
    2. Un-comment the `trusted-publishing` job.
3. Remove the `CARGO_TOKEN` [repository secret](https://github.com/PRO-2684/devctl/settings/secrets/actions).
4. Revoke the API token on [crates.io](https://crates.io/settings/tokens).

## 📥 Installation

### Using [`binstall`](https://github.com/cargo-bins/cargo-binstall)

```shell
cargo binstall devctl
```

### Downloading from Releases

Navigate to the [Releases page](https://github.com/PRO-2684/devctl/releases) and download respective binary for your platform. Make sure to give it execute permissions.

### Compiling from Source

```shell
cargo install devctl
```

## 💡 Examples

TODO

## 📖 Usage

TODO

## 🎉 Credits

TODO

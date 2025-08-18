# Contributing to me3

- [Contributing to me3](#contributing-to-me3)
  - [Development Workspace Setup](#development-workspace-setup)
  - [Testing](#testing)
    - [Unit Testing](#unit-testing)
    - [Live Debugging](#live-debugging)
  - [Formatting and Linting Code](#formatting-and-linting-code)
  - [Code Policy](#code-policy)
  - [Notes for maintainers](#notes-for-maintainers)

## Development Workspace Setup

This project uses the latest stable Rust version with `RUSTC_BOOTSTRAP=1` to access select unstable upstream features.

Set `RUSTC_BOOTSTRAP=1` when building, testing, or when using `rust-analyzer`.

The provided VS Code `.vscode/settings.json` config sets this environment variable automatically.

## Testing

Currently me3 can be tested by running its test of unit suites, or by injecting me3-mod-host into a game and performing live debugging.
Instructions are listed below for both.

### Unit Testing

Unit tests for the entire project can be ran with cargo, optionally specifying a package.

```shell
> $ cargo test [--package me3-mod-host]
```

### Live Debugging

me3 comes with a [Visual Studio Code](https://code.visualstudio.com/) workspace that contains a debugger launch configuration using [LLDB](https://lldb.llvm.org/) and [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb).
It will automatically start ELDEN RING, inject me3-mod-host, and attach a debugger.

You will also need to tell vscode where to find your copy of the game by creating a workspace settings file.
See `.vscode/settings.json.example` for how to do this.

Every first launch in a vscode session may result in an error about missing Rust/Cargo tasks, if this happens you may ignore it and build `me3` manually or install the `rust-analyzer` plugin and open a `.rs` file to initialize the extension.

## Formatting and Linting Code

This project uses [rustfmt](https://github.com/rust-lang/rustfmt) to handle formatting, and
contributions to its code are expected to be formatted with `rustfmt` using the
settings in [rustfmt.toml](rustfmt.toml).

## Code Policy

Code contributed to this project should follow the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html) as much as
possible (even if this project is an application instead of a library).

## Notes for maintainers

me3 uses a variety of hosted services, ensure you've been onboarded to the following:

- Cloudflare
- Google Search Console
- Google Analytics
- ReadTheDocs
- Sentry.io
- Crowdin.com

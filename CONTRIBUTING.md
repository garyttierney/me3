# Contributing to me3

- [Contributing to me3](#contributing-to-me3)
  - [Development Workspace Setup](#development-workspace-setup)
  - [Testing](#testing)
    - [Unit Testing](#unit-testing)
    - [Live Debugging](#live-debugging)
  - [Formatting and Linting Code](#formatting-and-linting-code)
  - [Code Policy](#code-policy)

## Development Workspace Setup

This project currently depends on the Rust `nightly` toolchain. The preferred way to install a `nightly` toolchain is via rustup:

```shell
> $ rustup toolchain install nightly
```

## Testing

Currently me3 can be tested by running its test of unit suites, or by injecting me3_host into a game and performing live debugging.
Instructions are listed below for both.

### Unit Testing

Unit tests for the entire project can be ran with cargo, optionally specifying a package.

```shell
> $ cargo test [--package me3_framework]
```

### Live Debugging

me3 comes with a [Visual Studio Code](https://code.visualstudio.com/) workspace that contains a debugger launch configuration using [LLDB](https://lldb.llvm.org/) and [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb).
It will automatically start Dark Souls 3, inject me3_host, and attach the debugger.

Unfortunately LLDB cannot be used out of the box to debug Dark Souls 3 as it interferes with anti-debug routines.
To work around this a [patched LLDB](https://tgathings.blob.core.windows.net/newcontainer/vscode-lldb.zip?sp=r&st=2022-09-21T19:20:49Z&se=2027-02-10T04:20:49Z&spr=https&sv=2021-06-08&sr=b&sig=lbO4T7%2B1mXDA%2FhvkaUhjvl9a3X6YW4x5E3imSYmjcQE%3D) is used that handles all exceptions as second-chance exceptions.

You will also need to tell vscode where to find your copy of the game by creating a workspace settings file.
See `.vscode/settings.json.example` for how to do this.

Every first launch in a vscode session may result in an error about missing Rust/Cargo tasks, if this happens you may ignore it and build `me3` manually or install the `rust-analyzer` plugin and open a `.rs` file to initialize the extension.

## Formatting and Linting Code

This project uses [rustfmt](https://github.com/rust-lang/rustfmt) to handle formatting, and
contributions to its code are expected to be formatted with `rustfmt` (within reason) using the
settings in [rustfmt.toml](rustfmt.toml).

This project uses [rust-clippy](https://github.com/rust-lang/rust-clippy) to handle linting, and
contributions are expected to be checked using the settings in [clippy.toml](clippy.toml).

>Note: `rustfmt` and `rust-clippy` each have many built-in defaults that will be deferred to in the
>absence of a corresponding rule in `rustfmt.toml`/`clippy.toml`.

## Code Policy

Code contributed to this project should follow the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html) as much as
possible (even if this project is an application instead of a library).

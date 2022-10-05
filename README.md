# me3

A framework for modifying software (particularly games) at runtime.

## Development

Building and testing `me3` requires an installation of Windows's C++ Build Tools (this is ehipped with an installation of Visual Studio).
It is recommended to use CLion or vscode to work on `me3` sources.

## Building

`me3` can be built using Cargo:

```shell
> $ cargo build [--release]
```

## Testing

### Unit Testing

Unit tests for the entire project can be ran with cargo, optionally specifying a package.

```shell
> $ cargo test [--package me3_framework]
```

### Live Testing

The vscode workspace for `me3` comes with an LLDB launch configuration that can be used to test the framework on a live game.
To use this, you'll need to acquire a patched [LLDB](https://tgathings.blob.core.windows.net/newcontainer/vscode-lldb.zip?sp=r&st=2022-09-21T19:20:49Z&se=2027-02-10T04:20:49Z&spr=https&sv=2021-06-08&sr=b&sig=lbO4T7%2B1mXDA%2FhvkaUhjvl9a3X6YW4x5E3imSYmjcQE%3D) and place it in `%userprofile%.vscode\extensions\vadimcn.vscode-lldb-*\lldb`.
The launch configuration will start the game, inject ScyllaHide, inject the `me3` host, and retuurn control back to the game.

You will also need to tell vscode where to find your copy of the game by creating a workspace settings file.
See `.vscode/settings.json.example` for how to do this.

Every first launch in a vscode session may result in an error about missing Rust/Cargo tasks, if this happens you may ignore it and build `me3` manually or install the `rust-analyzer` plugin and open a `.rs` file to initialize the extension.
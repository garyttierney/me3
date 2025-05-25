# Getting started with me3

This guide will help you get me3 installed and set up your first mod profile for ELDEN RING.

## Installation

### :fontawesome-brands-windows:  Windows

The easiest way to install me3 on Windows is by using the installer provided with each release.

1.  **Download the installer**:

    - Navigate to [GitHub releases](https://github.com/garyttierney/me3/releases/latest).
    - Find and download the `me3_installer` exe file.

    ???+ tip "Choosing a Release"

        It's recommended to use the latest stable release unless you have a specific reason to use an older version or a pre-release.

2.  **Run the Installer**:

    - Run the downloaded installer file.
    - Follow the on-screen instructions

### :fontawesome-brands-linux:  Linux

!!! todo "Linux Installation Guide"

## Creating a mod profile

A **Mod Profile** tells me3 which mods to load and how to load them. These profiles are defined in TOML files.

1.  **Create a mod profile configuration file**:
    Create a new text file with a `.me3.toml` extension. For example, `elden_ring_essentials.me3.toml`.

2.  **Define the profile**:
    Add the following content to your file, modifying it to suit your needs:

    ```toml
    # filepath: elden_ring_essentials.me3.toml
    profileVersion = "v1"

    [[packages]]
    id = "my-cool-texture-pack" # A unique name for this package
    source = "mods/MyCoolTexturePack/" # Path to the folder containing asset overrides.
                                       # This path is relative to the profile file, unless
                                       # specified as absolute.

    [[natives]]
    path = "mods/MyAwesomeMod.dll" # Path to the mod's DLL file

    # You can add more packages and natives
    # [[packages]]
    # id = "another-package"
    # source = "mods/AnotherPackage/"

    # [[natives]]
    # path = "mods/AnotherNative.dll"
    # optional = true
    ```

    ???+ tip "Understanding Paths"

        Any paths referenced in a mod profile (`source` in `[[packages]]` and `path` in `[[natives]]`) are relative to the location of the `.me3.toml` file itself

### Key concepts

You can define two main types of mod entries:

*   **Packages (`[[packages]]`)**: Use this to override game assets. Each package entry points to a directory (`source`) containing files that will replace the game's original files. The `id` gives your package a unique name that other packages can use to create a load order.

*   **Natives (`[[natives]]`)**: Use this to load custom DLL files (`.dll`) that inject new code or functionality into the game. Each native entry specifies the `path` to the DLL.


???+ info "Advanced profile configuration"

    The mod profile system is flexible and allows multiple profiles to specify dependencies between their packages and natives. For a complete list of all available options and their detailed descriptions, please    refer to the [configuration reference](./configuration-reference.md).

## Running your mod profile

!!! todo "Launcher guide"

## What's next?


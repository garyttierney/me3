# Installation

This guide provides step-by-step instructions to install `me3`, a mod loader for FROMSOFTWARE games. By the end of this guide you will have `me3` running on your system and be able to use mod profiles with ELDEN RING.

## Running the installer

=== ":fontawesome-brands-windows: Windows"

    The easiest way to install me3 on Windows is by using the installer provided with each release. This method ensures that all necessary files are correctly placed and configured on your system.

    <h3>1. Download the installer</h3>

    First, you'll need to download the installer from the official source. Navigate to the [me3 GitHub releases page](https://github.com/garyttierney/me3/releases/latest), which lists all available versions.

    Once you've selected a release, look for the `me3_installer.exe` file within its "Assets" section and download it.

    ??? warning "Browser security warnings (Click to open)"

        Your web browser might display a warning when downloading executable (`.exe`) files, suggesting that the file could be harmful. If you are downloading directly from the official `me3` GitHub repository, you can generally trust the file. Choose an option like "Keep" or "Download anyway" (the exact wording varies by browser). Always verify that the download source is `https://github.com/garyttierney/me3/`.

    <h3>2. Run the Installer</h3>

    After the `me3_installer.exe` file has finished downloading, locate it in your Downloads folder (or wherever you saved it) and double-click it to start the installation wizard.

    The installation wizard will then guide you through the setup. After choosing the install location, click "Install" to begin copying files. A progress bar will show the installation status, and once complete, a final screen will appear. Click "Finish" to close the installer.

=== ":fontawesome-brands-linux: Linux"

    me3 ships a shell-script installer for Linux that downloads the portable installation from GitHub, extracts the files to the correct locations, can be ran as a traditional one-line installer:

    <h3>1. Run the installer script</h3>

    ```bash
    curl --proto '=https' --tlsv1.2 -sSfL https://github.com/garyttierney/me3/releases/latest/download/installer.sh | sh
    ```

    <h3>2. Add the me3 binary to PATH</h3>

    Ensure that `me3` is available on your PATH by checking if `me3 info` is successful. If not, update the `PATH` environment variable to include `$HOME/.local/bin`.

## Verifying the installation

me3 will create a set of empty profiles for ELDEN RING by default.
Check the installation is working correctly by having me3 launch an empty profile either from a command-line or by double-clicking a .me3 file in the Windows shell:

```shell
> $ me3 launch --auto-detect -p eldenring-default
```

See `me3 launch --help` for information about the `auto-detect` parameters and more.

## What's next?

Check the [configuration reference](../configuration-reference.md) and [profile setup guide](./creating-mod-profiles.md) for information on how to get started using mods with me3.

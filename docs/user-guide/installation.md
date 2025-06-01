# Installation

This guide provides step-by-step instructions to install `me3`, a mod loader for FROMSOFTWARE games. By the end of this guide you will have `me3` running on your system and be able to use mod profiles with ELDEN RING.

## Running the installer

=== ":fontawesome-brands-windows: Windows"

    The easiest way to install me3 on Windows is by using the installer provided with each release. This method ensures that all necessary files are correctly placed and configured on your system.

    ### 1. Download the installer

    First, you'll need to download the installer from the official source. Navigate to the [me3 GitHub releases page](https://github.com/garyttierney/me3/releases/latest), which lists all available versions.

    ??? tip "Choosing a release (click to open)"

        It's recommended to download the latest stable release for the most up-to-date features and bug fixes. The latest release is always the most stable. Pre-release versions (e.g., alpha, beta, release candidates) are available for testing newer features but might be less stable. Only choose a pre-release if you are comfortable with potentially encountering bugs or if you need a specific feature not yet in a stable version.

    Once you've selected a release, look for the `me3_installer.exe` file within its "Assets" section and download it.

    ??? warning "Browser security warnings (Click to open)"

        Your web browser might display a warning when downloading executable (`.exe`) files, suggesting that the file could be harmful. If you are downloading directly from the official `me3` GitHub repository, you can generally trust the file. Choose an option like "Keep" or "Download anyway" (the exact wording varies by browser). Always verify that the download source is `https://github.com/garyttierney/me3/`.

    ### 2. Run the Installer

    After the `me3_installer.exe` file has finished downloading, locate it in your Downloads folder (or wherever you saved it) and double-click it to start the installation wizard.

    ??? tip "(Optional) Verifying the installer (Click to open)"
        For an added layer of security, you can verify the integrity and provenance of the downloaded `me3_installer.exe` using GitHub CLI attestations. This process checks cryptographic signatures to confirm the file was published by the `me3` developers and hasn't been altered since its publication.

        To do this, you'll need to have the [GitHub CLI (`gh`)](https://cli.github.com/) installed. Open your terminal or command prompt, navigate to the directory where you downloaded `me3_installer.exe`, and run the following command:

        ```bash
        gh attestation verify me3_installer.exe --repo garyttierney/me3
        ```


    The installation wizard will then guide you through the setup. After choosing the install location, click "Install" to begin copying files. A progress bar will show the installation status, and once complete, a final screen will appear. Click "Finish" to close the installer.

=== ":fontawesome-brands-linux: Linux"

    !!! todo "Linux Installation Guide"

## Verifying the installation

me3 will create a set of empty profiles for ELDEN RING and ELDEN RING NIGHTREIGN by default.
Check the installation is working correctly by having me3 launch an empty profile either from a command-line or by double-clicking a .me3 file in the Windows shell:

```shell
> $ me3 launch --auto-detect -p eldenring-default
```

See `me3 launch --help` for information about the `auto-detect` parameters and more.

## What's next?

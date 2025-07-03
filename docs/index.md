---
comments: false
hide:
  - navigation
  - toc
---

# Welcome to me3

**me<sup>3</sup>** is a framework designed for runtime modification of games, with a focus on ELDEN RING and other titles from FROMSOFTWARE.

[Download :fontawesome-solid-download:](https://github.com/garyttierney/me3/releases/latest){ .md-button .md-button--primary }

## Installation

=== ":fontawesome-brands-windows: Windows"

    **One-click installer:**

    Get the latest me3_installer.exe from [GitHub releases](https://github.com/garyttierney/me3/releases/latest) and follow the installation wizard.

    **Manual setup:**

    1. Download the [Windows portable distribution](https://github.com/garyttierney/me3/releases/latest)
    2. Extract it to a local directory (i.e. excluded from OneDrive or similar software) of your choosing.

=== ":fontawesome-brands-linux: Linux / Steam Deck"

    **One-line installer:**
    ```bash
    curl --proto '=https' --tlsv1.2 -sSfL https://github.com/garyttierney/me3/releases/latest/download/installer.sh | sh
    ```

    **Manual setup:**

    1. Download the [Linux portable distribution](https://github.com/garyttierney/me3/releases/latest)
    2. Extract it to a local directory:
       ```bash
       tar -xzf me3-linux-amd64.tar.gz
       cd me3-linux-amd64
       ./bin/me3 --windows-binaries-dir ./bin/win64 info
       ```

=== ":fontawesome-brands-apple: macOS"

    me3 supports macOS via [CrossOver®](https://www.codeweavers.com/crossover). Follow the Windows installation steps within your CrossOver environment.

## Quick Start Guide

### 1. Installation

Choose your platform above and follow the installation steps.

### 2. Setting up mod profiles

- [Creating mod profiles](user-guide/creating-mod-profiles.md) - Learn how to download and configure mods.
- [Configuration reference](configuration-reference.md) - Complete configuration options

### 3. Run a mod profile

Run the `.me3` profile you've configured, or launch a default profile from the start menu (Windows) or command-line:

```shell
me3 launch --auto-detect -p eldenring-default
```

## Need help?

- **First time user?** Start with our [user guide](user-guide/installation.md)
- **Having issues?** Check our [troubleshooting guide](user-guide/troubleshooting.md)
- **Found a bug?** [Report it](https://github.com/garyttierney/me3/discussions/categories/bug-reports)
- **Want a feature?** [Request it](https://github.com/garyttierney/me3/discussions/categories/ideas)

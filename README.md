<!-- markdownlint-disable no-inline-html first-line-h1 -->
<a name="readme-top"></a>

<br />
<div align="center">
  <p align="center">
    <img src="distribution/assets/me3.png" alt="me3 icon" />
  </p>
  <h2 align="center">me<sup>3</sup></h2>

  <p align="center">
    <strong>
    A framework for modifying and instrumenting games.
    <br />
    <a href="https://me3.readthedocs.io/"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="https://github.com/garyttierney/me3/discussions/categories/bug-reports">Report Bug</a>
    ·
    <a href="https://github.com/garyttierney/me3/discussions/categories/ideas">Request Feature</a>
    </strong>
  </p>

[![Discussions][discussions-shield]][discussions-url]
[![MIT + Apache-2.0 License][license-shield]][license-url]
![GitHub Downloads (all assets, all releases)][downloads-badge]
![GitHub commits since latest release][commits-badge]

</div>


- [About The Project](#about-the-project)
  - [Supported platforms](#supported-platforms)
  - [Supported games](#supported-games)
- [Installation](#installation)
- [Developer Quickstart](#developer-quickstart)
  - [Prerequisites](#prerequisites)
  - [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)
- [Contact](#contact)
- [Acknowledgments](#acknowledgments)

<!-- ABOUT THE PROJECT -->

## About The Project

me3 is a tool that extends the functionality of FROMSOTWARE games.

### Supported platforms

- Windows
- Linux via Proton
- macOS via [CrossOver®](https://www.codeweavers.com/crossover)

### Supported games

- ELDEN RING
- ELDEN RING NIGHTREIGN
- Armored Core VI: Fires of Rubicon
- Sekiro: Shadows Die Twice

## Installation

> [!IMPORTANT]
> Follow the [user guide](https://me3.readthedocs.io/en/latest/#quickstart)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->

## Developer Quickstart

### Prerequisites

- Cargo
  - Windows: download and run [rustup‑init.exe][rustup-installer] then follow the onscreen instructions.
  - Linux:

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

- Visual Studio C++ Build Tools
  - Windows: download and run [vs_BuildTools.exe][buildtools-installer] then follow the onscreen instructions.
  - Linux: Acquire the Windows SDK using `xwin`

    ```bash
    cargo install xwin && xwin --accept-license splat --output ~/.xwin
    ```

    And configure Cargo to link with lld-link and use the binaries from xwin in `~/.cargo/config.toml`

    ```toml
    [target.x86_64-pc-windows-msvc]
    linker = "lld-link"
    runner = "wine"
    rustflags = [
      "-Lnative=/home/gtierney/.xwin/crt/lib/x86_64",
      "-Lnative=/home/gtierney/.xwin/sdk/lib/um/x86_64",
      "-Lnative=/home/gtierney/.xwin/sdk/lib/ucrt/x86_64"
    ]
    ```

### Usage

1. Clone the repo

   ```sh
   git clone https://github.com/garyttierney/me3.git
   ```

2. Build the binaries

   ```sh
   cargo build [--release]
   ```

3. Attach the sample host DLL to your game

   ```sh
   cargo run -p me3-cli -- launch -g elden-ring
   ```

   <p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTRIBUTING -->

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md)

<!-- LICENSE -->

## License

With the exception of the [me3 logo](distribution/assets/me3.ico), this project is distributed under the terms of both the Apache Software License 2.0 and MIT License. See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for more information.

The me3 logo is not available under any license - all rights are reserved.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTACT -->

## Contact

Project Link: [https://github.com/garyttierney/me3](https://github.com/garyttierney/me3)

Discussions Board: [https://github.com/garyttierney/me3/discussions](https://github.com/garyttierney/me3/discussions)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ACKNOWLEDGMENTS -->

## Acknowledgments

<!-- markdown-link-check-disable -->
- [Mod Engine](https://github.com/katalash/ModEngine/tree/master/DS3ModEngine) - prior art for runtime modification of FROMSOFTWARE games.
- [Mod Organizer 2](https://github.com/ModOrganizer2/modorganizer/) - inspiration for the VFS framework.
- [Elden Ring Reforged](https://www.nexusmods.com/eldenring/mods/541) - provided invaluable feedback on the end-user perspective
- [Dassav](https://github.com/dasaav-dsv) - work on compatibility across a variety of FROMSOFTWARE titles.
- [Skadi](https://twitter.com/Skadi_sbw) - [me3 icon](./distribution/assets/me3.png) artwork
<!-- markdown-link-check-enable -->

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->

[rustup-installer]: https://static.rust-lang.org/dist/rust-1.87.0-x86_64-pc-windows-msvc.msi
[buildtools-installer]: https://aka.ms/vs/17/release/vs_BuildTools.exe
[discussions-shield]: https://img.shields.io/github/discussions/garyttierney/me3
[discussions-url]: https://github.com/garyttierney/me3/discussions
[contributors-shield]: https://img.shields.io/github/contributors/garyttierney/me3.svg?style=flat
[contributors-url]: https://github.com/garyttierney/me3/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/garyttierney/me3.svg?style=flat
[forks-url]: https://github.com/garyttierney/me3/network/members
[license-shield]: https://img.shields.io/badge/license-MIT%2FApache--2.0-green?style=flat
[license-url]: https://github.com/garyttierney/me3/blob/main/LICENSE-APACHE
[downloads-badge]: https://img.shields.io/github/downloads/garyttierney/me3/total
[commits-badge]: https://img.shields.io/github/commits-since/garyttierney/me3/latest

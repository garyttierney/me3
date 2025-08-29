# Known issues & FAQ

This section lists common issues and frequently asked questions based on real user feedback and GitHub reports. For troubleshooting steps, see the [troubleshooting guide](troubleshooting.md).

## FAQ

### Where are the mod profile folders?

The default mod profile folders are managed under `$HOME/.config/me3/profiles` on Linux and `%LOCALAPPDATA%\garyttierney\me3\config\profiles` on Windows

### The launcher won't start. What should I do?

Double-check your config file, antivirus settings, and see the [troubleshooting guide](troubleshooting.md).

### How do I install mods?

See the documentation on [creating mod profiles](./creating-mod-profiles.md)

### Where can I find my config file?

The global configuration file for me3 can be found in `$HOME/.config/me3/me3.toml` on Linux and `%LOCALAPPDATA%\garyttierney\me3\config\me3.toml` on Windows.

### How do I use a custom game path with me3?

The `me3 launch` command can be used to point to a custom game executable. For example:

```shell
> $ me3 launch --skip-steam-init --exe-path="C:/game-archive/eldenring.exe"
```

## Known Issues

### (Steam Deck) Game won't launch when game is installed to an SD card

!!! bug "me3 fails to find the compatprefix for games installed to an SD card"
!!! success "Move the game installation to main storage or create a symlink to the compat folder in your Steam library"

### me3 is quarantined by anti-virus software

!!! bug "Some antivirus software may flag the launcher or mod host as malicious."
!!! success "Add an exception for the launcher/mod host in your antivirus. Download only from official sources."

### Game is still running in Steam after exiting from the menu

!!! bug "The game or launcher processes may not always terminate cleanly"
!!! success "Manually end lingering game processes (e.g. via Task Manager on Windows)."

---

For more help, visit the [Troubleshooting Guide](troubleshooting.md) or join the community discussions.

# Troubleshooting

Encountering issues when first setting up mods is a common experience. This section will guide you through diagnosing and resolving some of the more frequent problems you might face with `me3`.

!!! warning "First check: the usual suspects"
    Before diving deeper, it's wise to quickly verify a few common sources of error. Often, the issue is a simple **typo** within your `.me3` file, an `id`, or a keyword like `packages` or `path`. Another frequent pitfall is **incorrect paths**. Remember that all `path`s for `[[packages]]` and `[[natives]]` are **relative** to the location of your `.me3` file, so ensure these accurately point to your mod files.

---

## Resources

- For a list of current bugs and common questions, see the [Known Issues & FAQ](./faq.md#known-issues).

## Common problems

### Anti-virus warnings

me3 binaries are now code-signed with a Certum certificate to reduce false positives. If your antivirus flags me3:

- Verify downloads are from the official [GitHub releases](https://github.com/garyttierney/me3/releases)
- Add the me3 installer and me3 installation directory to your antivirus exclusions

### Game fails to launch

- Ensure Steam is running before launching me3
- Double-check the paths listed in your .me3 file
- (Windows) Run (++windows+r++) `me3 info` to check installation was successful
- (Linux) verify that `windows_binaries_dir` is set in your configuration file (`~/.config/me3`)

## Still running into problems?

File a bug report or ask for help on the [discussions board](https://github.com/garyttierney/me3/discussions/)

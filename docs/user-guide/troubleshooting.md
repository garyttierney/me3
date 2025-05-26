# Troubleshooting

Encountering issues when first setting up mods is a common experience. This section will guide you through diagnosing and resolving some of the more frequent problems you might face with `me3`.

!!! warning "First check: the usual suspects"
    Before diving deeper, it's wise to quickly verify a few common sources of error. Often, the issue is a simple **typo** within your `.me3.toml` file, an `id`, or a keyword like `source` or `path`. Another frequent pitfall is **incorrect paths**; remember that all `source` paths for `[[packages]]` and `path` entries for `[[natives]]` are **relative** to the location of your `.me3.toml` file, so ensure these accurately point to your mod files.

## :fontawesome-solid-bug: Game crashes at startup or during play

Crashes often point to a specific mod and often the most effective way to troubleshoot them is by isolating the problematic mod.

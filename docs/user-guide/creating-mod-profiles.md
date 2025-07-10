# Creating mod profiles

A **Mod Profile** tells me3 which mods to load and how to load them. This guide will take you through downloading mods, setting up a local mod directory, and creating the mod profile.

We'll setup the following DLL mods: [Fast Launch](https://www.nexusmods.com/eldenringnightreign/mods/30), [Nightreign Alt Saves](https://www.nexusmods.com/eldenringnightreign/mods/4) and [Disable Chromatic Aberration](https://www.nexusmods.com/eldenringnightreign/mods/67).

For content replacement we'll use [Fun Is Allowed](https://www.nexusmods.com/eldenringnightreign/mods/49) and [Geralt of Rivia over Wylder](https://www.nexusmods.com/eldenringnightreign/mods/63).

## Step 1: Prepare your mod directory

- Decide where you're going to store your mod files. By default me3 stores them in `%LOCALAPPDATA%/garyttierney/me3/config/profiles` or `$HOME/.config/me3/profiles`, but a `.me3` file can live anywhere besides a network drive.
- Create a folder named `mod` to store your downloaded mod files.

## Step 2: Add your mods

- Place asset files (e.g. `regulation.bin`, `parts/` folders) in `mod`.
- Place `.dll` files in `natives`.
- For easier management, you can use subfolders in `mod` and reference them with separate `[[packages]]` entries in your profile. This makes it much easier to add/remove/update individual mods.

!!! tip "Understanding Paths"
    Any paths referenced in a mod profile (`path` in `[[packages]]` and `[[natives]]`) are relative to the location of the `.me3` file itself.
    You can store your mod files in any path that you choose as long as you use the correct path in the `.me3` file.

For the example profile we should download the FIA mod and place its `regulation.bin` file into our `mod` folder, then download the Geralt mod and place the `parts` folder into our `mod` folder. For the DLLs, we place each of the DLLs from the downloaded mods into the `natives` folder.

!!! warning "Native mod compatibility"
    Some native mods may have their own restrictions or requirements on how they're configured. Make sure to consult the documentation for each mod.

## Step 3: Create your mod profile

Create a new file (e.g. `myprofile.me3`) in your `Mods` folder with the following content:

```toml
profileVersion = "v1"

[[supports]]
game = "nightreign"

[[packages]]
id = "nightmods"
path = 'mod'

[[natives]]
path = 'natives/DisableChromaticAberration.dll'

[[natives]]
path = 'natives/SkipIntroLogos.dll'

[[natives]]
path = 'natives/nightreign_alt_saves.dll'
```

This profile declares an asset replacement package named `nightmods` (using all files in the `mod` folder) and lists each `.dll` mod in the `natives` folder. We also declare that our profile supports NIGHTREIGN so me3 knows the game to configure when use double-click to launch.

## Step 4: Run the profile

Now the profile has been setup it's time to run it. Users on Windows can simply double-click the `.me3` file to launch the game with their mods, while users on Linux need to run the profile using the cross-platform CLI:

```shell
> $ me3 launch --auto-detect -p myprofile.me3
```

## Step 5: Play the modded game

![image](https://github.com/user-attachments/assets/9da0bf73-695d-4f0b-af83-2c88e6328fd3)

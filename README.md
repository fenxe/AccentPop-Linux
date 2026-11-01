# AccentPop

AccentPop is a lightweight keyboard accent selector for Linux, written in Rust
using GTK4.

It lets you access accented characters and alternative symbols by holding
configured keys, without needing to remember complicated keyboard shortcuts
or character codes.

## Features

- Hold a configured key to open an accent selection popup.
- Select characters using the keyboard or mouse.
- Navigate available characters with the keyboard.
- Supports uppercase and lowercase character selection.
- Supports custom accent and symbol mappings.
- Customizable popup appearance through CSS.
- Default configuration files are generated automatically on first launch.
- Existing user configuration is preserved.
- Designed for Hyprland.

## Requirements

- Arch Linux or another compatible Linux distribution.
- Hyprland.
- GTK4.
- gtk4-layer-shell.
- Rust and Cargo (for building from source).
- wtype for character input.
- Access to the physical keyboard input device.
- Access to /dev/uinput.

The user running AccentPop must have appropriate permissions to access the
keyboard input device and /dev/uinput. On Arch Linux, membership in the
input group is generally required. Log out and back in after changing group
membership.

## Configuration

On first launch, AccentPop creates its configuration directory:

~/.config/accentpop/

It generates two default files:

- accents.txt
- style.css

### accents.txt

Defines the keys that trigger the accent selector and the characters
available for each key.

You can edit this file to customize your accent and symbol mappings.

### style.css

Controls the appearance of the accent selection popup.

You can customize its colors, spacing, buttons, and other supported GTK CSS
properties.

Your existing configuration files are not overwritten when AccentPop starts.

## Building from source

Install the required build dependencies on Arch Linux:

    sudo pacman -S --needed base-devel rust cargo gtk4 gtk4-layer-shell pkgconf wtype

Clone the repository:

    git clone https://github.com/fenxe/AccentPop-Linux.git
    cd accentpop

Build:

    cargo build --release

Run:

    ./target/release/accentpop

Replace USERNAME with the GitHub account or organization hosting the
repository.

## License

AccentPop is distributed under the MIT License. See LICENSE.

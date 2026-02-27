# raydvd

`raydvd` is a tiny desktop DVD logo overlay for Linux, written in Rust with raylib.

![screenshot](./screenshot.png)

It opens a transparent, borderless, click-through window, bounces a DVD logo around the screen, changes color on each bounce, and flashes on corner hits.

## Features

- Transparent, undecorated, always-on-top overlay window
- Mouse click-through behavior
- System tray item (`💿 raydvd`) with Quit action
- Random color changes on bounce
- Corner-hit flash effect
- Optional center-path trace mode

## Install

Build requirements:

- `cmake`
- C toolchain (`cc`, `c++`)
- OpenGL/X11 development libraries on Linux (for raylib via GLFW)

Ubuntu/Debian example:

```bash
sudo apt-get update
sudo apt-get install -y cmake build-essential \
  libasound2-dev libgl1-mesa-dev libx11-dev libxcursor-dev \
  libxext-dev libxinerama-dev libxi-dev libxrandr-dev
```

From crates.io (after publish):

```bash
cargo install raydvd
```

From source:

```bash
git clone https://github.com/krisfur/raydvd
cd raydvd
cargo run --release
```

## Usage

```bash
raydvd [OPTIONS]
```

Options:

- `-s, --speed <FLOAT>`: speed multiplier, must be `> 0` (default: `1.0`)
- `-c, --corner <FLOAT>`: corner-hit margin in pixels, must be `>= 0` (default: `20`)
- `-t, --trace`: draw the traveled path of the logo center

Examples:

```bash
raydvd -s 2.5
raydvd -s 69 -c 5 -t
```

## Controls

- Tray icon menu: Quit
- `Ctrl+C` while window is focused: quit

## Notes

- Tray integration is only enabled on Linux; on other platforms the app runs without a tray icon.
- On Wayland, tray visibility depends on your StatusNotifier host (for example, Waybar tray module).
- Transparency/click-through behavior can vary slightly by compositor configuration.

## License

MIT

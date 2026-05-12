# mousejiggler-rs

A lean Rust reimplementation of [Mouse Jiggler](https://github.com/arkane-systems/mousejiggler) for Windows. Virtually jiggles the mouse cursor so the computer appears not idle — useful for keeping screensavers, sleep, and presence indicators at bay.

No WinForms, no .NET runtime, no extra DLLs. A single native `.exe` that talks directly to the Win32 API via the `windows` crate.

## Features

- Four jiggle patterns: **Normal**, **Zen** (no-op, keeps the system awake without moving the cursor), **Circle**, **Linear**
- Configurable period (1–10800 seconds) and distance multiplier (1–120)
- Optional random timer variation
- Smart-pause: stops jiggling while the user is actively moving the mouse
- Keeps the display and system awake via `SetThreadExecutionState`
- System tray icon with context menu, minimize-to-tray, and start-minimized
- Per-user settings persisted to `HKCU\Software\ArkaneSystems\MouseJiggler` (registry — wire-compatible with the upstream C# build)
- Single-instance enforcement via a named mutex
- Per-monitor DPI awareness
- CLI flags for headless / scripted launches

## Build

Requires a recent Rust toolchain (edition 2024) and the MSVC build tools.

```
cargo build --release
```

The release profile is tuned for small binaries (`opt-level = "z"`, LTO, single codegen unit, `panic = "abort"`, symbol stripping). The resulting executable is at `target/release/mousejiggler-rs.exe`.

`build.rs` compiles `resources/app.rc` (icon, manifest, dialog templates) and embeds it into the binary via the `embed-resource` crate.

## Usage

Launch with no arguments for the GUI. CLI flags override the stored settings for this run only — they do not get written back to the registry.

```
mousejiggler-rs [options]

  -j, --jiggle               Start with jiggling enabled.
  -m, --minimized            Start minimized.
  -o, --mode <MODE>          Normal | Zen | Circle | Linear
  -r, --random               Random timer variation.
  -s, --seconds <N>          Jiggle interval, 1..=10800.
  -d, --distance <N>         Distance multiplier, 1..=120.
  -g, --settings             Open with the settings panel visible.
  -?, -h, --help             Show help.
      --version              Show version.
```

Example — start minimized to tray and immediately begin a 30-second Circle jiggle:

```
mousejiggler-rs -j -m -o Circle -s 30
```

`--help` and `--version` print to the parent console when launched from a terminal (the process is otherwise a `windows_subsystem = "windows"` GUI app, so it does not allocate a console of its own).

## Project layout

```
src/
  main.rs              Entry point: CLI parse, single-instance, message loop
  cli.rs               Hand-rolled flag parser
  jiggle.rs            Patterns, SendInput, smart-pause, execution state
  settings.rs          Registry-backed persistence
  single_instance.rs   Named-mutex guard
  tray.rs              Shell_NotifyIcon wrapper
  ui_main.rs           Main dialog + control wiring
  ui_about.rs          About dialog
  rng.rs               Small PRNG for random-timer mode
  ids.rs               Resource / control IDs, clamps, defaults
  util.rs              Wide-string helpers, console attach/detach
resources/
  app.rc, resource.h   Dialog templates, version info, icon, accelerators
  app.manifest         DPI awareness, common controls v6, requestedExecutionLevel
  icon.ico
build.rs               Invokes embed-resource to compile app.rc
```

## Relationship to upstream Mouse Jiggler

This is a from-scratch Rust port that aims to be behaviourally compatible with [ArkaneSystems/MouseJiggler](https://github.com/arkane-systems/mousejiggler):

- Same registry path and value names — settings written by either build are readable by the other.
- Same single-instance mutex name.
- Same CLI surface, jiggle patterns, and clamp ranges.

You can drop this `.exe` in place of the C# build without losing your saved preferences.

## License

Apache-2.0. See the upstream project for the original C# implementation and its license.

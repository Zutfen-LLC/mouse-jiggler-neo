# mousejiggler-rs

A lean Rust implementation of Mouse Jiggler for Windows. Virtually jiggles the mouse cursor so the computer appears not idle - useful for keeping screensavers, sleep, and presence indicators at bay.

No WinForms, no .NET runtime, no extra DLLs. A single native `.exe` that talks directly to the Win32 API via the `windows` crate.

## Features

- Four jiggle patterns: **Normal**, **Zen** (no-op, keeps the system awake without moving the cursor), **Circle**, **Linear**
- Configurable period (1-10800 seconds) and distance multiplier (1-120)
- Optional random timer variation
- Smart-pause: stops jiggling while the user is actively moving the mouse
- Keeps the display and system awake via `SetThreadExecutionState`
- System tray icon with context menu, minimize-to-tray, and start-minimized
- Per-user settings persisted to `HKCU\Software\Zutfen-LLC\MouseJiggler` (registry)
- Single-instance enforcement via a named mutex
- Per-monitor DPI awareness
- CLI flags for headless / scripted launches

## Build

Requires a recent Rust toolchain (edition 2024) and the MSVC build tools.

```sh
cargo build --release
```

The release profile is tuned for small binaries (`opt-level = "z"`, LTO, single codegen unit, `panic = "abort"`, symbol stripping). The resulting executable is at `target/release/mousejiggler-rs.exe`.

`build.rs` compiles `resources/app.rc` (icon, manifest, dialog templates) and embeds it into the binary via the `embed-resource` crate.

## Usage

Launch with no arguments for the GUI. CLI flags override the stored settings for this run only - they do not get written back to the registry.

```text
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

Example - start minimized to tray and immediately begin a 30-second Circle jiggle:

```sh
mousejiggler-rs -j -m -o Circle -s 30
```

`--help` and `--version` print to the parent console when launched from a terminal (the process is otherwise a `windows_subsystem = "windows"` GUI app, so it does not allocate a console of its own).

## Project layout

```text
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

## Compatibility notes

This build keeps a few historical behaviors for compatibility:

- Same registry value names and behavior, but under `HKCU\Software\Zutfen-LLC\MouseJiggler`
- Same CLI surface, jiggle patterns, and clamp ranges

Changing the registry vendor path means this build does not share saved preferences with older builds unless you migrate that key.

## License

Apache-2.0. See [LICENSE](LICENSE).

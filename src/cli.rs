//! Hand-rolled CLI parser.
//!
//! Flags:
//!   -j --jiggle             Start with jiggling enabled.
//!   -m --minimized          Start minimized.
//!   -o --mode <m>           Jiggle mode: Normal | Zen | Circle | Linear.
//!   -r --random             Random timer variation.
//!   -s --seconds <n>        Jiggle interval (1..=10800).
//!   -d --distance <n>       Distance multiplier (1..=120).
//!   -g --settings           Start with settings panel displayed.
//!   -? -h --help            Show help.
//!   --version               Show version.

use crate::ids::{DISTANCE_MAX, DISTANCE_MIN, PERIOD_MAX, PERIOD_MIN};
use crate::jiggle::Mode;
use crate::util::console_println;

#[derive(Default, Debug)]
pub struct Args {
    pub jiggle: bool,
    pub minimized: Option<bool>,
    pub mode: Option<Mode>,
    pub random: Option<bool>,
    pub seconds: Option<u32>,
    pub distance: Option<u32>,
    pub settings_panel: bool,
}

pub enum ParseOutcome {
    Run(Args),
    PrintAndExit(i32),
    Error(String),
}

pub fn parse() -> ParseOutcome {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut a = Args::default();
    let mut i = 0;
    while i < raw.len() {
        let arg = raw[i].as_str();
        match arg {
            "-j" | "--jiggle" => a.jiggle = true,
            "-m" | "--minimized" => a.minimized = Some(true),
            "-r" | "--random" => a.random = Some(true),
            "-g" | "--settings" => a.settings_panel = true,
            "-?" | "-h" | "--help" => {
                print_help();
                return ParseOutcome::PrintAndExit(0);
            }
            "--version" => {
                print_version();
                return ParseOutcome::PrintAndExit(0);
            }
            "-o" | "--mode" => {
                let Some(v) = raw.get(i + 1) else {
                    return ParseOutcome::Error(format!("{arg} requires a value"));
                };
                let Some(m) = Mode::parse(v) else {
                    return ParseOutcome::Error(format!("Invalid jiggle mode: {v}"));
                };
                a.mode = Some(m);
                i += 1;
            }
            "-s" | "--seconds" => {
                let Some(v) = raw.get(i + 1) else {
                    return ParseOutcome::Error(format!("{arg} requires a value"));
                };
                let n: u32 = v.parse().map_err(|_| ()).and_then(|n: u32| Ok(n)).unwrap_or(0);
                if n < PERIOD_MIN {
                    return ParseOutcome::Error(
                        "Period cannot be shorter than 1 second.".into(),
                    );
                }
                if n > PERIOD_MAX {
                    return ParseOutcome::Error(
                        "Period cannot be longer than 10800 seconds.".into(),
                    );
                }
                a.seconds = Some(n);
                i += 1;
            }
            "-d" | "--distance" => {
                let Some(v) = raw.get(i + 1) else {
                    return ParseOutcome::Error(format!("{arg} requires a value"));
                };
                let n: u32 = v.parse().unwrap_or(0);
                if n < DISTANCE_MIN {
                    return ParseOutcome::Error(
                        "Distance multiplier cannot be less than 1.".into(),
                    );
                }
                if n > DISTANCE_MAX {
                    return ParseOutcome::Error(
                        "Distance multiplier cannot be greater than 120.".into(),
                    );
                }
                a.distance = Some(n);
                i += 1;
            }
            other => {
                return ParseOutcome::Error(format!("Unknown argument: {other}"));
            }
        }
        i += 1;
    }
    ParseOutcome::Run(a)
}

pub fn print_help() {
    let text = "\
Mouse Jiggler (Rust) — virtually jiggles the mouse, making the computer seem not idle.

Usage:
  mousejiggler-rs [options]

Options:
  -j, --jiggle               Start with jiggling enabled.
  -m, --minimized            Start minimized.
  -o, --mode <Normal|Zen|Circle|Linear>
                             Start with the specified jiggle mode enabled.
  -r, --random               Start with random timer variation enabled.
  -s, --seconds <N>          Jiggle interval in seconds (1..=10800).
  -d, --distance <N>         Distance multiplier (1..=120).
  -g, --settings             Start with settings panel displayed.
  -?, -h, --help             Show help and usage information.
      --version              Show version information.";
    console_println(text);
}

pub fn print_version() {
    console_println(concat!("mousejiggler-rs ", env!("CARGO_PKG_VERSION")));
}

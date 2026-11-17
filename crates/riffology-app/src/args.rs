//! Command line options. `riffology --help` prints them.

use std::path::PathBuf;

/// The options the app runs with.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Args {
    /// Replay this WAV file instead of opening the audio input.
    pub replay: Option<PathBuf>,
    /// Play the replayed file to the default output too.
    pub monitor: bool,
    /// Replay the file again and again.
    pub looped: bool,
    /// Seconds to wait after the window opens before the replay starts.
    pub replay_delay: f64,
    /// The audio input device name; the default input when None.
    pub device: Option<String>,
    /// The input channel the guitar is on, 0 the first.
    pub channel: usize,
    /// Open a MIDI input: Some("") for the first port, or part of a port name.
    pub midi: Option<String>,
    /// A chart to play (a .json chart or a .mid file).
    pub chart: Option<PathBuf>,
    /// Write every note event and every drawn mark to this JSON file.
    pub trace: Option<PathBuf>,
    /// Quit after this many seconds.
    pub exit_after: Option<f64>,
    /// The rux theme name.
    pub theme: Option<String>,
    /// The tuning name.
    pub tuning: Option<String>,
    /// Open the window without taking the foreground.
    pub background: bool,
    pub help: bool,
}

/// The text `--help` prints.
pub const HELP: &str = "riffology [options]
  --replay <file.wav>   replay a WAV file in real time instead of the audio input
  --monitor             play the replayed file to the speakers too
  --loop                replay the file again and again
  --replay-delay <s>    wait this long before the replay starts (for screen recordings)
  --device <name>       audio input device (default: the system default input)
  --channel <n>         input channel the guitar is on, 0 the first (default 0)
  --midi [name]         also listen to a MIDI input (the first port, or one whose name contains <name>)
  --chart <file>        play against a chart (.json chart or .mid file)
  --tuning <name>       Standard, Drop D, DADGAD, Open G, Open D, Half step down
  --theme <name>        dark-neumorphic (default) or light-neumorphic
  --trace <file.json>   write every note event and every lit cell to a file
  --exit-after <s>      quit after this many seconds
  --background          open without taking the foreground";

/// Parses the arguments after the program name.
/// @param argv - the arguments
pub fn parse(argv: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut i = 0;
    let value = |i: &mut usize, name: &str| -> Result<String, String> {
        *i += 1;
        argv.get(*i).cloned().ok_or_else(|| format!("{name} needs a value"))
    };
    while i < argv.len() {
        match argv[i].as_str() {
            "--replay" => a.replay = Some(value(&mut i, "--replay")?.into()),
            "--monitor" => a.monitor = true,
            "--loop" => a.looped = true,
            "--replay-delay" => {
                a.replay_delay = value(&mut i, "--replay-delay")?.parse().map_err(|_| "--replay-delay needs seconds")?
            }
            "--device" => a.device = Some(value(&mut i, "--device")?),
            "--channel" => a.channel = value(&mut i, "--channel")?.parse().map_err(|_| "--channel needs a number")?,
            "--midi" => {
                a.midi = Some(
                    argv.get(i + 1).filter(|v| !v.starts_with("--")).cloned().inspect(|_| i += 1).unwrap_or_default(),
                )
            }
            "--chart" => a.chart = Some(value(&mut i, "--chart")?.into()),
            "--trace" => a.trace = Some(value(&mut i, "--trace")?.into()),
            "--exit-after" => {
                a.exit_after = Some(value(&mut i, "--exit-after")?.parse().map_err(|_| "--exit-after needs seconds")?)
            }
            "--theme" => a.theme = Some(value(&mut i, "--theme")?),
            "--tuning" => a.tuning = Some(value(&mut i, "--tuning")?),
            "--background" => a.background = true,
            "--help" | "-h" => a.help = true,
            other => return Err(format!("unknown option {other}\n\n{HELP}")),
        }
        i += 1;
    }
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn options_parse() {
        let a = parse(&s(&["--replay", "a.wav", "--midi", "--trace", "t.json", "--exit-after", "12.5"])).unwrap();
        assert_eq!(a.replay, Some("a.wav".into()));
        assert_eq!(a.midi, Some(String::new()));
        assert_eq!(a.exit_after, Some(12.5));
        assert_eq!(parse(&s(&["--midi", "Fishman"])).unwrap().midi.as_deref(), Some("Fishman"));
        assert!(parse(&s(&["--nope"])).is_err());
    }
}

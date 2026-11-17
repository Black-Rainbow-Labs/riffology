//! A small WAV reader: 16 bit and 24 bit PCM and 32 bit float, any channel count. Returns one channel.

/// The samples of one channel and the sample rate.
#[derive(Debug, Clone)]
pub struct Wav {
    pub samples: Vec<f32>,
    pub rate: u32,
}

/// Reads a WAV file and returns channel `channel` (0 the first).
/// @param path - the file
/// @param channel - the channel to keep
pub fn read_wav(path: &std::path::Path, channel: usize) -> Result<Wav, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_wav(&bytes, channel).map_err(|e| format!("{}: {e}", path.display()))
}

/// Parses WAV bytes.
/// @param bytes - the file contents
/// @param channel - the channel to keep
pub fn parse_wav(bytes: &[u8], channel: usize) -> Result<Wav, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let (mut fmt, mut data) = (None, None);
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = &bytes[pos + 8..(pos + 8 + len).min(bytes.len())];
        match id {
            b"fmt " => fmt = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        pos += 8 + len + (len & 1);
    }
    let fmt = fmt.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    if fmt.len() < 16 {
        return Err("short fmt chunk".into());
    }
    let format = u16::from_le_bytes([fmt[0], fmt[1]]);
    let channels = u16::from_le_bytes([fmt[2], fmt[3]]) as usize;
    let rate = u32::from_le_bytes(fmt[4..8].try_into().unwrap());
    let bits = u16::from_le_bytes([fmt[14], fmt[15]]);
    if channel >= channels {
        return Err(format!("channel {channel} asked of a {channels} channel file"));
    }
    let width = (bits / 8) as usize;
    let frame = width * channels;
    let samples = data
        .chunks_exact(frame)
        .map(|f| decode(&f[channel * width..(channel + 1) * width], format, bits))
        .collect::<Result<Vec<f32>, String>>()?;
    Ok(Wav { samples, rate })
}

/// Decodes one sample.
/// @param b - the sample's bytes
/// @param format - 1 PCM, 3 float, 0xFFFE extensible (read by bit depth)
/// @param bits - bits per sample
fn decode(b: &[u8], format: u16, bits: u16) -> Result<f32, String> {
    match (format, bits) {
        (3, 32) | (0xFFFE, 32) => Ok(f32::from_le_bytes(b.try_into().unwrap())),
        (1, 32) => Ok(i32::from_le_bytes(b.try_into().unwrap()) as f32 / 2_147_483_648.0),
        (_, 16) => Ok(i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0),
        (_, 24) => Ok((((b[2] as i32) << 24 | (b[1] as i32) << 16 | (b[0] as i32) << 8) >> 8) as f32 / 8_388_608.0),
        _ => Err(format!("unsupported WAV format {format} with {bits} bits")),
    }
}

/// Writes mono 16 bit PCM, for tests and evidence files.
/// @param path - the file
/// @param samples - the samples, -1 to 1
/// @param rate - the sample rate
pub fn write_wav16(path: &std::path::Path, samples: &[f32], rate: u32) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_file_reads_back() {
        let dir = std::env::temp_dir().join("riffology-wav-test.wav");
        let s: Vec<f32> = (0..480).map(|i| (i as f32 / 480.0) - 0.5).collect();
        write_wav16(&dir, &s, 48_000).unwrap();
        let w = read_wav(&dir, 0).unwrap();
        std::fs::remove_file(&dir).unwrap();
        assert_eq!((w.rate, w.samples.len()), (48_000, 480));
        assert!(w.samples.iter().zip(&s).all(|(a, b)| (a - b).abs() < 1e-4));
    }
}

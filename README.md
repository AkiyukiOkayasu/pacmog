# pacmog

[![Cargo](https://img.shields.io/crates/v/pacmog.svg)](https://crates.io/crates/pacmog)  
[![Documentation](https://docs.rs/pacmog/badge.svg)](https://docs.rs/pacmog)  
![Tests](https://github.com/AkiyukiOkayasu/pacmog/actions/workflows/ci.yml/badge.svg)  

pacmog is a decoding library for the PCM file.  
Designed for use in playing the PCM file embedded in microcontroller firmware.  
Rust has an include_bytes! macro to embed the byte sequence in the program. Using it, PCM files can be embedded in firmware and used for playback.  

| Format          | Status |
| :---            | :---: |
| WAV 16bit | ✅ |
| WAV 24bit | ✅ |
| WAV 32bit | ✅ |
| WAV 32bit float | ✅ |
| WAV 64bit float | ✅ |
| IMA ADPCM | ✅ |
| AIFF 16bit | ✅ |
| AIFF 24bit | ✅ |
| AIFF 32bit | ✅ |
| AIFF 32bit float | ✅ |
| AIFF 64bit float | ✅ |

## Example

```bash
cargo run --example beep
```

### Low-level: Read the sample at an arbitrary position

```Rust
use pacmog::PcmReader;

let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
let mut input = &wav[..];
let reader = PcmReader::new(&mut input)?;
let specs = reader.get_pcm_specs();
let num_samples = specs.num_samples;
let num_channels = specs.num_channels;
println!("PCM info: {:?}", specs);

for sample in 0..num_samples {
    for channel in 0..num_channels {
        let sample_value: f32 = reader.read_sample(channel, sample)?;
        println!("{}", sample_value);
    }
}
```

### High-level: PcmPlayer

```Rust
use pacmog::{PcmPlayer, PcmReader};

let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
let mut input = &wav[..];
let reader = PcmReader::new(&mut input)?;
let mut player = PcmPlayer::new(reader);
let specs = player.reader.get_pcm_specs();
let num_samples = specs.num_samples;
player.set_loop_playing(true);
let mut buffer: [f32; 2] = [0.0f32; 2];
let buf = buffer.as_mut_slice();

for sample in 0..num_samples {
   player.get_next_frame(buf)?;
}
```

## Benchmark

```bash
cargo criterion
```

## no_std

pacmog works with no_std by default.  
No setup is needed.  

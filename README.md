# pacmog

![Tests](https://github.com/AkiyukiOkayasu/pacmog/actions/workflows/ci.yml/badge.svg)

Library for decoding PCM files embedded with include_bytes!.
Designed for use in playing PCM files embedded in microcontroller firmware.  
pacmog works with no_std by default.

| Format          | Status |
| :---            | :---: |
| WAV 16bit       | ✅ |
| WAV 24bit       | ✅ |
| WAV 32bit       | ✅ |
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

## Test

```bash
cargo test
```

## Benchmark

```bash
cargo criterion
```

## no_std

pacmog works with no_std by default.  
No setup is needed.  

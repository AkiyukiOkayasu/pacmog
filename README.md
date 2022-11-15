# pacmog

include_bytes!で埋め込んだWAVEファイルをデコードするためのライブラリ。  
マイコンのファームウェアに効果音を埋め込むなどの利用を想定。  
no_stdで使えるように[nom](https://github.com/Geal/nom)でWAVEのパースをしている。  

| Format          | Status |
| :---            | :---: |
| WAV 16bit       | ✅ |
| WAV 24bit       | ✅ |
| WAV 32bit       | ✅ |
| WAV 32bit float | ✅ |
| WAV 64bit float | ✅ |
| IMA ADPCM | - |
| μ-law | - |
| A-law | - |
| AIFF 16bit | ✅ |
| AIFF 24bit | ✅ |
| AIFF 32bit | ✅ |
| AIFF 32bit float | - |

## TODO

- no_stdで動作させる
  - nomのmany0とかが使えなくなる

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

use criterion::{criterion_group, criterion_main, Criterion};
use pacmog::PcmReader;

fn parse_wav(c: &mut Criterion) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    c.bench_function("Parse WAV 16bit", |b| {
        b.iter(|| {
            let _reader = PcmReader::new(wav);
        })
    });
}

fn read_sample(c: &mut Criterion) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    let reader = PcmReader::new(wav);
    c.bench_function("Read a sample 16bit", |b| {
        b.iter(|| {
            let _sample = reader.read_sample(0, 0).unwrap();
        })
    });
}

criterion_group!(benches, parse_wav, read_sample);
criterion_main!(benches);

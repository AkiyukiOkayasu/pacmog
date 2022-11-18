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
    let pcm_specs = reader.get_pcm_specs();
    c.bench_function("Read a sample 16bit", |b| {
        b.iter(|| {
            for sample in 0..48000 {
                for channel in 0..pcm_specs.num_channels as u32 {
                    let _s = reader.read_sample(channel, sample).unwrap();
                }
            }
        })
    });
}

criterion_group!(benches, parse_wav, read_sample);
criterion_main!(benches);

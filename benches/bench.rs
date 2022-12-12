use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pacmog::imaadpcm::ImaAdpcmPlayer;
use pacmog::PcmReader;

fn parse_wav(c: &mut Criterion) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    c.bench_function("Parse WAV 16bit", |b| {
        b.iter(|| {
            let _reader = PcmReader::new(black_box(wav));
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

fn parse_decode_ima_adpcm(c: &mut Criterion) {
    let data = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_4bit_IMAADPCM.wav");
    let mut player = ImaAdpcmPlayer::new(data);
    let mut buffer: [i16; 2] = [0i16, 0i16];

    c.bench_function("Decode IMA-ADPCM", |b| {
        let buf = buffer.as_mut_slice();
        b.iter(|| player.get_next_frame(buf))
    });
}

criterion_group!(benches, parse_wav, read_sample, parse_decode_ima_adpcm);
criterion_main!(benches);

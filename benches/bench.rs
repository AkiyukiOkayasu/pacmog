use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pacmog::imaadpcm::{ImaAdpcmPlayer, I1F15};
use pacmog::{PcmPlayer, PcmReader};

fn parse_wav(c: &mut Criterion) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    c.bench_function("Parse WAV 16bit", |b| {
        b.iter(|| {
            let _reader = PcmReader::new(black_box(wav)).unwrap();
        })
    });
}

fn read_sample(c: &mut Criterion) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    let reader = PcmReader::new(wav).unwrap();
    let pcm_specs = reader.get_pcm_specs();
    c.bench_function("Read a sample 16bit", |b| {
        b.iter(|| {
            for sample in 0..48000 {
                for channel in 0..pcm_specs.num_channels {
                    let _s = reader.read_sample(channel, sample).unwrap();
                }
            }
        })
    });
}

fn player(c: &mut Criterion) {
    let data = include_bytes!("../tests/resources/MLKDream.wav");
    let reader = PcmReader::new(data).unwrap();
    let mut player = PcmPlayer::new(reader);
    let mut buffer: [f32; 2] = [0.0, 0.0];
    let buf = buffer.as_mut_slice();

    c.bench_function("PcmPlayer", |b| {
        b.iter(|| {
            player.set_position(0).unwrap();
            for _ in 0..1_000_000 {
                player.get_next_frame(buf).unwrap();
            }
        })
    });
}

fn parse_decode_ima_adpcm(c: &mut Criterion) {
    let data = include_bytes!("../tests/resources/Sine440Hz_2ch_48000Hz_4bit_IMAADPCM.wav");
    let mut buffer: [I1F15; 2] = [I1F15::ZERO, I1F15::ZERO];

    c.bench_function("Decode IMA-ADPCM", |b| {
        let mut player = ImaAdpcmPlayer::new(data);
        let buf = buffer.as_mut_slice();
        b.iter(|| {
            // player = ImaAdpcmPlayer::new(data);
            player.rewind();
            for _ in 0..192000 {
                //4sec
                player.get_next_frame(buf).unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    parse_wav,
    read_sample,
    parse_decode_ima_adpcm,
    player
);
criterion_main!(benches);

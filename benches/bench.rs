#![feature(test)]

extern crate test;
use pacmo::PcmReader;
use test::Bencher;

#[bench]
fn bench_one(b: &mut Bencher) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    b.iter(|| {
        let reader = PcmReader::read_bytes(wav);
        for i in 0..100 {
            let _sample = reader.read_sample(0, i).unwrap();
        }
    });
}

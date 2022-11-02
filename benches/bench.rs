#![feature(test)]

extern crate test;
use pacmo::PcmReader;
use test::Bencher;

#[bench]
fn parse_wav(b: &mut Bencher) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    b.iter(|| {
        let _reader = PcmReader::read_bytes(wav);
    });
}

#[bench]
fn read_sample(b: &mut Bencher) {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    let reader = PcmReader::read_bytes(wav);
    b.iter(|| {
        let _sample = reader.read_sample(0, 0).unwrap();
    });
}

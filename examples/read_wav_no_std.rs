//! Read wav file in no_std environment.

// #![no_std]

use pacmog::PcmReader;

fn main() {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    let mut input = &wav[..];
    let reader = PcmReader::new(&mut input).unwrap();
    for sample in 0..48000 {
        let _s = reader.read_sample(0, sample);
    }
}

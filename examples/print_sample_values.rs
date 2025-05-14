use pacmog::PcmReader;
use std::{fs, io::Write};

fn main() {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_64FP.wav");
    println!("Wave length in bytes: {}", wav.len());
    let mut input = &wav[..];
    let reader = PcmReader::new(&mut input).unwrap();
    println!("PCM spec: {:?}", reader.get_pcm_specs());

    let mut file = fs::File::create("sinewave.txt").unwrap();

    // Print and export 3000 samples values to txt file
    for i in 0..3000 {
        let s = reader.read_sample(0, i).unwrap();
        println!("{i}: {s}");
        write!(file, "{s}f32, ").unwrap();
    }
}

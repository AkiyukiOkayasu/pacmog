use approx::assert_relative_eq;
use pacmog::PcmReader;

const SINEWAVE: [f32; 10] = [
    0f32,
    0.05130394f32,
    0.10243774f32,
    0.15323183f32,
    0.20351772f32,
    0.2531287f32,
    0.3019002f32,
    0.34967047f32,
    0.39628112f32,
    0.44157755f32,
];

#[test]
fn wav_16bit() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_16.wav");
    let reader = PcmReader::read_bytes(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(
            sample,
            SINEWAVE[i as usize],
            epsilon = f32::EPSILON * 200f32
        );
    }
}

#[test]
fn wav_24bit() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_24.wav");
    let reader = PcmReader::read_bytes(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize], epsilon = f32::EPSILON * 10f32);
    }
}

#[test]
fn wav_32bit() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32.wav");
    let reader = PcmReader::read_bytes(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn wav_32bit_float() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32FP.wav");
    let reader = PcmReader::read_bytes(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn wav_64bit_float() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_64FP.wav");
    let reader = PcmReader::read_bytes(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn aiff_16bit() {
    let aiff = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_16.aif");
    let reader = PcmReader::read_bytes(aiff);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(
            sample,
            SINEWAVE[i as usize],
            epsilon = f32::EPSILON * 200f32
        );
    }
}

#[test]
fn aiff_24bit() {
    let aiff = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_24.aif");
    let reader = PcmReader::read_bytes(aiff);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize], epsilon = f32::EPSILON * 10f32);
    }
}

#[test]
fn aiff_32bit() {
    let aiff = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32.aif");
    let reader = PcmReader::read_bytes(aiff);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

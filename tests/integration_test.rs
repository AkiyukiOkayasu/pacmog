use approx::assert_relative_eq;
use fixed::types::I1F15;
use pacmog::{AudioFormat, PcmReader};

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
fn fixed_test() {
    let hoge = I1F15::from_num(0.5);
    let fuga = I1F15::from_num(0.1);
    assert_eq!(hoge, 0.5);
    assert_eq!(fuga.to_ne_bytes(), I1F15::from_num(0.1).to_ne_bytes());
    let aaa = hoge.checked_mul(fuga).unwrap();
    assert_eq!(aaa.to_ne_bytes(), I1F15::from_num(0.05).to_ne_bytes());
}

#[test]
fn wav_linearpcm_specs() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_16.wav");
    let reader = PcmReader::new(wav);
    let spec = reader.get_pcm_specs();
    assert_eq!(spec.bit_depth, 16);
    assert_eq!(spec.audio_format, AudioFormat::LinearPcmLe);
    assert_eq!(spec.num_channels, 1);
    assert_eq!(spec.sample_rate, 48000);
    assert_eq!(spec.num_samples, 240000);
}

#[test]
fn aiff_linearpcm_specs() {
    let data = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_16.aif");
    let reader = PcmReader::new(data);
    let spec = reader.get_pcm_specs();
    assert_eq!(spec.bit_depth, 16);
    assert_eq!(spec.audio_format, AudioFormat::LinearPcmBe); //Big endian
    assert_eq!(spec.num_channels, 1);
    assert_eq!(spec.sample_rate, 48000);
    assert_eq!(spec.num_samples, 240000);
}

#[test]
fn wav_float32_specs() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32FP.wav");
    let reader = PcmReader::new(wav);
    let spec = reader.get_pcm_specs();
    assert_eq!(spec.bit_depth, 32);
    assert_eq!(spec.audio_format, AudioFormat::IeeeFloatLe); //Little endian
    assert_eq!(spec.num_channels, 1);
    assert_eq!(spec.sample_rate, 48000);
    assert_eq!(spec.num_samples, 240000);
}

#[test]
fn aiff_float32_specs() {
    let data = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32FP.aif");
    let reader = PcmReader::new(data);
    let spec = reader.get_pcm_specs();
    assert_eq!(spec.bit_depth, 32);
    assert_eq!(spec.audio_format, AudioFormat::IeeeFloatBe); //Big endian
    assert_eq!(spec.num_channels, 1);
    assert_eq!(spec.sample_rate, 48000);
    assert_eq!(spec.num_samples, 240000);
}

#[test]
fn wav_16bit() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_16.wav");
    let reader = PcmReader::new(wav);

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
    let reader = PcmReader::new(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize], epsilon = f32::EPSILON * 10f32);
    }
}

#[test]
fn wav_32bit() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32.wav");
    let reader = PcmReader::new(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn wav_32bit_float() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32FP.wav");
    let reader = PcmReader::new(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn wav_64bit_float() {
    let wav = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_64FP.wav");
    let reader = PcmReader::new(wav);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn aiff_16bit() {
    let aiff = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_16.aif");
    let reader = PcmReader::new(aiff);

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
    let reader = PcmReader::new(aiff);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize], epsilon = f32::EPSILON * 10f32);
    }
}

#[test]
fn aiff_32bit() {
    let aiff = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32.aif");
    let reader = PcmReader::new(aiff);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn aiff_32bit_float() {
    let aiff = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_32FP.aif");
    let reader = PcmReader::new(aiff);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn aiff_64bit_float() {
    let aiff = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_64FP.aif");
    let reader = PcmReader::new(aiff);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(sample, SINEWAVE[i as usize]);
    }
}

#[test]
fn ima_adpcm_4bit() {
    let data = include_bytes!("./resources/Sine440Hz_1ch_48000Hz_4bit_IMAADPCM.wav");
    let reader = PcmReader::new(data);

    for i in 0..10 {
        let sample = reader.read_sample(0, i).unwrap();
        assert_relative_eq!(
            sample,
            SINEWAVE[i as usize],
            epsilon = f32::EPSILON * 200f32
        );
    }
}

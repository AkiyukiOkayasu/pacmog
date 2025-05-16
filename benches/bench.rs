use pacmog::imaadpcm::{ImaAdpcmPlayer, I1F15};
use pacmog::{PcmPlayer, PcmReader};

static WAV_SINE_MONO_48_16_BIT: &[u8] =
    include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
static WAV_SINE_MONO_48_32_FP: &[u8] =
    include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_32FP.wav");
static TANK: &[u8] = include_bytes!("../tests/resources/Tank_Low17.wav");
static MLK_DREAM: &[u8] = include_bytes!("../tests/resources/MLKDream.wav");
static IMAADPCM_SINE_MONO_48: &[u8] =
    include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_4bit_IMAADPCM.wav");
static IMAADPCM_SINE_STEREO_48: &[u8] =
    include_bytes!("../tests/resources/Sine440Hz_2ch_48000Hz_4bit_IMAADPCM.wav");

#[divan::bench(args = [&WAV_SINE_MONO_48_16_BIT, &WAV_SINE_MONO_48_32_FP, &TANK, &MLK_DREAM, &IMAADPCM_SINE_MONO_48, &IMAADPCM_SINE_STEREO_48])]
fn parse_wav(input: &[u8]) {
    let mut input = input;
    let _reader = PcmReader::new(&mut input).unwrap();
}

#[divan::bench(args = [&WAV_SINE_MONO_48_16_BIT, &WAV_SINE_MONO_48_32_FP, &TANK, &MLK_DREAM])]
fn read_sample(input: &[u8]) {
    let mut input = input;
    let reader = PcmReader::new(&mut input).unwrap();
    let pcm_specs = reader.get_pcm_specs();

    for sample in 0..192000 {
        for channel in 0..pcm_specs.num_channels {
            let _s: f32 = reader.read_sample(channel, sample).unwrap();
        }
    }
}

#[divan::bench(
    types = [f32, f64],
    args = [&WAV_SINE_MONO_48_16_BIT, &WAV_SINE_MONO_48_32_FP, &TANK, &MLK_DREAM]
)]
fn linear_pcm_player<T: num_traits::float::Float>(input: &[u8]) {
    let mut input = input;
    let reader = PcmReader::new(&mut input).unwrap();
    let mut player = PcmPlayer::new(reader);
    let mut buffer: [T; 2] = [T::zero(), T::zero()];
    let buf = buffer.as_mut_slice();

    player.set_position(0).unwrap();
    for _ in 0..192_000 {
        player.get_next_frame(buf).unwrap();
    }
}

#[divan::bench(args = [&IMAADPCM_SINE_MONO_48, &IMAADPCM_SINE_STEREO_48])]
fn ima_adpcm_player(input: &[u8]) {
    let mut input = input;
    let mut buffer: [I1F15; 2] = [I1F15::ZERO, I1F15::ZERO];
    let mut player = ImaAdpcmPlayer::new(&mut input).unwrap();
    let buf = buffer.as_mut_slice();

    player.rewind();
    for _ in 0..192000 {
        //4sec
        player.get_next_frame(buf).unwrap();
    }
}

fn main() {
    divan::main();
}

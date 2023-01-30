//! Read IMA-ADPCM file in no_std environment.

#![no_std]
use fixed::types::I1F15;
use pacmog::imaadpcm::ImaAdpcmPlayer;

fn main() {
    let data = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_4bit_IMAADPCM.wav");
    let mut player = ImaAdpcmPlayer::new(data);
    let mut buffer: [I1F15; 2] = [I1F15::ZERO, I1F15::ZERO];
    let b = buffer.as_mut_slice();

    for _ in 0..48000 {
        player.get_next_frame(b).unwrap();
    }
}

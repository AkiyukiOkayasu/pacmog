//! Play a sample mono ADPCM file.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use pacmog::imaadpcm::ImaAdpcmPlayer;
use std::sync::mpsc;

fn main() {
    let data = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_4bit_IMAADPCM.wav");
    let mut player = ImaAdpcmPlayer::new(data);
    let mut buffer: [i16; 2] = [0i16, 0i16];

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    println!("Default output device: {:?}", device.name());

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);
    let channels = config.channels() as usize;

    println!("PCM spec: {:?}", player.reader.get_pcm_specs());

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let (complete_tx, complete_rx) = mpsc::sync_channel::<()>(1);

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let buf = buffer.as_mut_slice();
                    match player.get_next_frame(buf) {
                        Ok(_) => {
                            for (_ch, sample) in frame.iter_mut().enumerate() {
                                *sample = buf[0] as f32 / i16::MAX as f32;
                            }
                        }
                        Err(e) => {
                            println!("{}", e);
                            let _result = complete_tx.try_send(());
                        }
                    }
                }
            },
            err_fn,
        )
        .unwrap();

    stream.play().unwrap();
    complete_rx.recv().unwrap();
    stream.pause().unwrap();
    println!("done")
}

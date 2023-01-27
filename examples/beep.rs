//! Play a sample WAV file.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use pacmog::PcmReader;
use std::sync::mpsc;

fn main() {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    println!("Wave length in bytes: {}", wav.len());

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    println!("Default output device: {:?}", device.name());

    let config = device.default_output_config().unwrap();
    println!("Default output config: {config:?}");
    let channels = config.channels() as usize;

    let reader = PcmReader::new(wav);
    let mut sample_index = 0;

    println!("PCM spec: {:?}", reader.get_pcm_specs());

    let err_fn = |err| eprintln!("an error occurred on stream: {err}");
    let (complete_tx, complete_rx) = mpsc::sync_channel::<()>(1);

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    for sample in frame.iter_mut() {
                        match reader.read_sample(0, sample_index) {
                            Err(_) => {
                                let _result = complete_tx.try_send(());
                            }
                            Ok(s) => {
                                *sample = s;
                            }
                        }
                    }
                    sample_index += 1;
                }
            },
            err_fn,
        )
        .unwrap();

    stream.play().unwrap();
    complete_rx.recv().unwrap();
    stream.pause().unwrap();
    println!("done");
}

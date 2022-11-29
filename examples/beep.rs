use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use pacmog::PcmReader;

fn main() {
    let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
    println!("Wave length in bytes: {}", wav.len());

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    println!("Default output device: {:?}", device.name());

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);
    let channels = config.channels() as usize;

    let reader = PcmReader::new(wav);
    let mut sample_index = 0;

    println!("PCM spec: {:?}", reader.get_pcm_specs());

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // write_data(data, channels, &mut next_value)
                for frame in data.chunks_mut(channels) {
                    for sample in frame.iter_mut() {
                        *sample = reader.read_sample(0, sample_index).unwrap();
                    }
                    sample_index += 1;
                }
            },
            err_fn,
        )
        .unwrap();
    stream.play().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1000));
}

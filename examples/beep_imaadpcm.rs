use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use pacmog::imaadpcm::ImaAdpcmPlayer;

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
    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // write_data(data, channels, &mut next_value)
                for frame in data.chunks_mut(channels) {
                    let buf = buffer.as_mut_slice();
                    player.get_next_frame(buf).unwrap();
                    for (ch, sample) in frame.iter_mut().enumerate() {
                        *sample = buf[ch] as f32 / i16::MAX as f32;
                    }
                }
            },
            err_fn,
        )
        .unwrap();
    stream.play().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(1000));
}

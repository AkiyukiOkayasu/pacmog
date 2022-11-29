use core::f32;

use anyhow::{bail, ensure};
use nom::error::Error;
use nom::number::complete::{
    be_f32, be_f64, be_i16, be_i24, be_i32, le_f32, le_f64, le_i16, le_i24, le_i32,
};
use nom::Finish;
use nom::{multi::many1, IResult};
use wav::WavFmtSpecs;

mod aiff;
pub mod imaadpcm;
mod wav;

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum AudioFormat {
    #[default]
    Unknown,
    LinearPcmLe,
    LinearPcmBe,
    IeeeFloatLe,
    IeeeFloatBe,
    ImaAdpcm,
}

/// PCMファイルの情報.
/// * 'audio_format' -
/// * 'num_channels' - Mono: 1, Stereo: 2
/// * 'sample_rate' - 48000Hz, 44100Hz and so on.
/// * 'bit_depth' - 16bit, 24bit, 32bit and so on.
/// * 'num_samples' - Number of samples per channel.
/// * 'ima_adpcm_num_block_align' - IMA-ADPCM only. IMA-ADPCMの1ブロックが何byteで構成されているか。
/// * 'ima_adpcm_num_samples_per_block' - IMA-ADPCM only. IMA-ADPCMの1ブロックに何サンプル記録されているか。
#[derive(Default, Debug, Clone)]
pub struct PcmSpecs {
    pub audio_format: AudioFormat,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub num_samples: u32,
    pub(crate) ima_adpcm_num_block_align: Option<u16>,
    pub(crate) ima_adpcm_num_samples_per_block: Option<u16>,
}

/// PCMファイルの低レベルな情報を取得するためのクラス
#[derive(Default)]
pub struct PcmReader<'a> {
    pub(crate) specs: PcmSpecs,
    pub(crate) data: &'a [u8],
}

impl<'a> PcmReader<'a> {
    fn parse_aiff(&mut self, input: &'a [u8]) -> IResult<&[u8], &[u8]> {
        let (input, v) = many1(aiff::parse_chunk)(input)?;

        for e in v {
            match e.id {
                aiff::ChunkId::Common => {
                    let (_, spec) = aiff::parse_comm(e.data)?;
                    dbg!(&spec);
                    self.specs = spec;
                }
                aiff::ChunkId::SoundData => {
                    let (data, _ssnd_block_info) = aiff::parse_ssnd(e.data)?; //TODO ssnd_block_infoのoffset, blockSizeが0でないときの対応追加
                    self.data = data;
                }
                aiff::ChunkId::FormatVersion => {}
                aiff::ChunkId::Marker => {}
                aiff::ChunkId::Instrument => {}
                aiff::ChunkId::MIDI => {}
                aiff::ChunkId::AudioRecording => {}
                aiff::ChunkId::ApplicationSpecific => {}
                aiff::ChunkId::Comment => {}
                aiff::ChunkId::Name => {}
                aiff::ChunkId::Author => {}
                aiff::ChunkId::Copyright => {}
                aiff::ChunkId::Annotation => {}
                aiff::ChunkId::Unknown => {}
            }
        }
        return Ok((input, &[]));
    }

    fn parse_wav(&mut self, input: &'a [u8]) -> IResult<&[u8], &[u8]> {
        //many1はallocが実装されていないと使えない。no_stdで使うなら逐次的に実行するべき。
        let (input, v) = many1(wav::parse_chunk)(input)?;
        let mut fmt_spec = WavFmtSpecs::default();

        for e in v {
            match e.id {
                wav::ChunkId::Fmt => {
                    let (_, spec) = wav::parse_fmt(e.data)?;
                    fmt_spec = spec;
                    self.specs.num_channels = fmt_spec.num_channels;
                    self.specs.sample_rate = fmt_spec.sample_rate;
                    self.specs.audio_format = fmt_spec.audio_format;
                    self.specs.bit_depth = fmt_spec.bit_depth;
                    if self.specs.audio_format == AudioFormat::ImaAdpcm {
                        self.specs.ima_adpcm_num_block_align = fmt_spec.ima_adpcm_num_block_align;
                        self.specs.ima_adpcm_num_samples_per_block =
                            fmt_spec.ima_adpcm_num_samples_per_block;
                    }
                }
                wav::ChunkId::Data => {
                    self.data = e.data;
                }
                wav::ChunkId::Fact => {}
                wav::ChunkId::IDv3 => {}
                wav::ChunkId::JUNK => {}
                wav::ChunkId::LIST => {}
                wav::ChunkId::PEAK => {}
                wav::ChunkId::Unknown => {}
            }
        }

        match self.specs.audio_format {
            AudioFormat::ImaAdpcm => {
                self.specs.num_samples =
                    imaadpcm::calc_num_samples_per_channel(self.data.len() as u32, &self.specs)
                        .unwrap();
            }
            AudioFormat::LinearPcmLe | AudioFormat::IeeeFloatLe => {
                self.specs.num_samples =
                    wav::calc_num_samples_per_channel(self.data.len() as u32, &self.specs).unwrap();
            }
            _ => {
                unreachable!();
            }
        }
        return Ok((input, &[]));
    }

    /// * 'input' - PCM data byte array
    pub fn new(input: &'a [u8]) -> Self {
        let file_length = input.len();

        let mut reader: PcmReader = Default::default();

        //WAVの場合
        if let Ok((input, riff)) = wav::parse_riff_header(input) {
            assert_eq!((file_length - 8) as u32, riff.size);
            if let Ok((_, _)) = reader.parse_wav(input) {
                return reader;
            }
        };

        //AIFFの場合
        if let Ok((input, aiff)) = aiff::parse_aiff_header(input) {
            assert_eq!((file_length - 8) as u32, aiff.size);
            if let Ok((_, _)) = reader.parse_aiff(input) {
                return reader;
            }
        };

        //WAVでもAIFFでもなかった場合
        panic!();
    }

    /// ファイル情報の取得
    pub fn get_pcm_specs(&self) -> PcmSpecs {
        self.specs.clone()
    }

    /// DATAチャンクを読んでサンプルを読みだす    
    /// フォーマットに関わらず+/-1の範囲に正規化された数を返す
    /// TODO f32以外Q15やQ23, f64などでも返せるようにしたい
    /// もしくはf32かf64を選択できるようにする
    /// 固定小数点の取得はread_raw_sample()的な関数とそのジェネリスクで対応するのがいいかもしれない
    pub fn read_sample(&self, channel: u32, sample: u32) -> anyhow::Result<f32> {
        ensure!(channel < self.specs.num_channels as u32, "Invalid channel");
        ensure!(sample < self.specs.num_samples, "Invalid sample");

        let byte_depth = self.specs.bit_depth as u32 / 8u32;
        let byte_offset = ((byte_depth * sample * self.specs.num_channels as u32)
            + (byte_depth * channel)) as usize;
        let data = &self.data[byte_offset..];
        decode_sample(&self.specs, data)
    }
}

/// DATAチャンクを読んでサンプルを読みだす    
/// フォーマットに関わらず+/-1の範囲に正規化された数を返す
/// TODO f32以外Q15やQ23, f64などでも返せるようにしたい
/// もしくはf32かf64を選択できるようにする
/// 固定小数点の取得はread_raw_sample()的な関数とそのジェネリスクで対応するのがいいかもしれない
pub(crate) fn decode_sample(specs: &PcmSpecs, data: &[u8]) -> anyhow::Result<f32> {
    match specs.audio_format {
        AudioFormat::Unknown => {
            bail!("Unknown audio format");
        }
        AudioFormat::LinearPcmLe => {
            match specs.bit_depth {
                16 => {
                    const MAX: u32 = 2u32.pow(15); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) = le_i16::<_, Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    return Ok(sample);
                }
                24 => {
                    const MAX: u32 = 2u32.pow(23); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) = le_i24::<_, Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    return Ok(sample);
                }
                32 => {
                    const MAX: u32 = 2u32.pow(31); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) = le_i32::<_, Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    return Ok(sample);
                }
                _ => bail!("Unsupported bit-depth"),
            }
        }
        AudioFormat::LinearPcmBe => {
            match specs.bit_depth {
                16 => {
                    const MAX: u32 = 2u32.pow(15); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) = be_i16::<_, Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    return Ok(sample);
                }
                24 => {
                    const MAX: u32 = 2u32.pow(23); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) = be_i24::<_, Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    return Ok(sample);
                }
                32 => {
                    const MAX: u32 = 2u32.pow(31); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) = be_i32::<_, Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    return Ok(sample);
                }
                _ => bail!("Unsupported bit-depth"),
            }
        }
        AudioFormat::IeeeFloatLe => {
            match specs.bit_depth {
                32 => {
                    //32bit float
                    let (_remains, sample) = le_f32::<_, Error<_>>(data).finish().unwrap();
                    return Ok(sample);
                }
                64 => {
                    //64bit float
                    let (_remains, sample) = le_f64::<_, Error<_>>(data).finish().unwrap();
                    return Ok(sample as f32); // TODO f32にダウンキャストするべきなのか検討
                }
                _ => bail!("Unsupported bit-depth"),
            }
        }
        AudioFormat::IeeeFloatBe => {
            match specs.bit_depth {
                32 => {
                    //32bit float
                    let (_remains, sample) = be_f32::<_, Error<_>>(data).finish().unwrap();
                    return Ok(sample);
                }
                64 => {
                    //64bit float
                    let (_remains, sample) = be_f64::<_, Error<_>>(data).finish().unwrap();
                    return Ok(sample as f32); // TODO f32にダウンキャストするべきなのか検討
                }
                _ => bail!("Unsupported bit-depth"),
            }
        }
        AudioFormat::ImaAdpcm => {
            bail!("IMA-ADPCM is not supported in decode_sample(). Use ImaAdpcmPlayer.")
        }
    }
}

/// PCMファイルを再生するために高レベルにまとめられたクラス
/// * 'reader' - PCMファイルの低レベル情報にアクセスするためのクラス
/// * 'reading_buffer' - 再生中のバッファー。get_next_frame()で使用する。
#[derive(Default)]
pub struct PcmPlayer<'a> {
    pub reader: PcmReader<'a>,
    reading_data: &'a [u8],
    loop_playing: bool,
}

impl<'a> PcmPlayer<'a> {
    /// * 'input' - PCM data byte array
    pub fn new(input: &'a [u8]) -> Self {
        let reader = PcmReader::new(input);
        let mut player = PcmPlayer {
            reader,
            reading_data: &[],
            loop_playing: false,
        };
        player.set_position(0);
        player
    }

    /// 再生位置のセット
    pub fn set_position(&mut self, sample: u32) {
        let byte_depth = self.reader.specs.bit_depth as u32 / 8u32;
        let byte_offset = (byte_depth * sample * self.reader.specs.num_channels as u32) as usize;
        self.reading_data = &self.reader.data[byte_offset..];
    }

    /// ループ再生の有効無効設定.
    /// true: loop enable
    /// false: loop disable
    pub fn set_loop_playing(&mut self, en: bool) {
        self.loop_playing = en;
    }

    /// 次のサンプル（全チャンネル）を取得.
    /// * 'out' - サンプルが書き込まれるバッファー
    pub fn get_next_frame(&mut self, out: &mut [f32]) -> anyhow::Result<()> {
        let byte_depth = self.reader.specs.bit_depth / 8;

        ensure!(
            out.len() >= self.reader.specs.num_channels as usize,
            "Invalid output buffer length"
        );

        if self.reading_data.len() <= 0 {
            if self.loop_playing {
                self.set_position(0);
            } else {
                bail!("Finished playing");
            }
        }

        for ch in 0..self.reader.specs.num_channels {
            let sample = decode_sample(
                &self.reader.specs,
                &self.reading_data[(ch * byte_depth) as usize..],
            );
            out[ch as usize] = sample.unwrap();
        }

        //update reading_data
        self.reading_data =
            &self.reading_data[(self.reader.specs.num_channels * byte_depth) as usize..];

        Ok(())
    }
}

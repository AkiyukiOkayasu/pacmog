use nom::error::Error;
use nom::number::complete::{
    be_f32, be_f64, be_i16, be_i24, be_i32, le_f32, le_f64, le_i16, le_i24, le_i32,
};
use nom::Finish;
use nom::{multi::many1, IResult};

mod aiff;
mod imaadpcm;
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
#[derive(Default, Debug, Clone)]
pub struct PcmSpecs {
    pub audio_format: AudioFormat,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub bit_depth: u16,
}

#[derive(Default)]
pub struct PcmReader<'a> {
    specs: PcmSpecs,
    data: &'a [u8],
}

impl<'a> PcmReader<'a> {
    fn parse_aiff(&mut self, input: &'a [u8]) -> IResult<&[u8], &[u8]> {
        let (input, v) = many1(aiff::parse_chunk)(input)?;

        for e in v {
            match e.id {
                aiff::ChunkId::Common => {
                    let (_, spec) = aiff::parse_comm(e.data)?;
                    println!("{:?}", spec);
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

        for e in v {
            match e.id {
                wav::ChunkId::Fmt => {
                    let (_, spec) = wav::parse_fmt(e.data)?;
                    self.specs = spec;
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
    pub fn read_sample(&self, channel: u32, sample: u32) -> Option<f32> {
        if channel >= self.specs.num_channels as u32 {
            return None;
        }

        let max_sample_size = self.data.len() / self.specs.num_channels as usize;
        if sample >= max_sample_size as u32 {
            return None;
        }

        match self.specs.audio_format {
            AudioFormat::Unknown => return None,
            AudioFormat::LinearPcmLe => {
                match self.specs.bit_depth {
                    16 => {
                        let byte_offset =
                            (2u32 * sample * self.specs.num_channels as u32) + (2u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        const MAX: u32 = 2u32.pow(15); //normalize factor: 2^(BitDepth-1)
                        let (_remains, sample) = le_i16::<_, Error<_>>(data).finish().unwrap();
                        let sample = sample as f32 / MAX as f32;
                        return Some(sample);
                    }
                    24 => {
                        let byte_offset =
                            (3u32 * sample * self.specs.num_channels as u32) + (3u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        const MAX: u32 = 2u32.pow(23); //normalize factor: 2^(BitDepth-1)
                        let (_remains, sample) = le_i24::<_, Error<_>>(data).finish().unwrap();
                        let sample = sample as f32 / MAX as f32;
                        return Some(sample);
                    }
                    32 => {
                        let byte_offset =
                            (4u32 * sample * self.specs.num_channels as u32) + (4u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        const MAX: u32 = 2u32.pow(31); //normalize factor: 2^(BitDepth-1)
                        let (_remains, sample) = le_i32::<_, Error<_>>(data).finish().unwrap();
                        let sample = sample as f32 / MAX as f32;
                        return Some(sample);
                    }
                    _ => return None,
                }
            }
            AudioFormat::LinearPcmBe => {
                match self.specs.bit_depth {
                    16 => {
                        let byte_offset =
                            (2u32 * sample * self.specs.num_channels as u32) + (2u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        const MAX: u32 = 2u32.pow(15); //normalize factor: 2^(BitDepth-1)
                        let (_remains, sample) = be_i16::<_, Error<_>>(data).finish().unwrap();
                        let sample = sample as f32 / MAX as f32;
                        return Some(sample);
                    }
                    24 => {
                        let byte_offset =
                            (3u32 * sample * self.specs.num_channels as u32) + (3u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        const MAX: u32 = 2u32.pow(23); //normalize factor: 2^(BitDepth-1)
                        let (_remains, sample) = be_i24::<_, Error<_>>(data).finish().unwrap();
                        let sample = sample as f32 / MAX as f32;
                        return Some(sample);
                    }
                    32 => {
                        let byte_offset =
                            (4u32 * sample * self.specs.num_channels as u32) + (4u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        const MAX: u32 = 2u32.pow(31); //normalize factor: 2^(BitDepth-1)
                        let (_remains, sample) = be_i32::<_, Error<_>>(data).finish().unwrap();
                        let sample = sample as f32 / MAX as f32;
                        return Some(sample);
                    }
                    _ => return None,
                }
            }
            AudioFormat::IeeeFloatLe => {
                match self.specs.bit_depth {
                    32 => {
                        //32bit float
                        let byte_offset =
                            (4u32 * sample * self.specs.num_channels as u32) + (4u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        let (_remains, sample) = le_f32::<_, Error<_>>(data).finish().unwrap();
                        return Some(sample);
                    }
                    64 => {
                        //64bit float
                        let byte_offset =
                            (8u32 * sample * self.specs.num_channels as u32) + (8u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        let (_remains, sample) = le_f64::<_, Error<_>>(data).finish().unwrap();
                        return Some(sample as f32); // TODO f32にダウンキャストするべきなのか検討
                    }
                    _ => {
                        return None;
                    }
                }
            }
            AudioFormat::IeeeFloatBe => {
                match self.specs.bit_depth {
                    32 => {
                        //32bit float
                        let byte_offset =
                            (4u32 * sample * self.specs.num_channels as u32) + (4u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        let (_remains, sample) = be_f32::<_, Error<_>>(data).finish().unwrap();
                        return Some(sample);
                    }
                    64 => {
                        //64bit float
                        let byte_offset =
                            (8u32 * sample * self.specs.num_channels as u32) + (8u32 * channel);
                        let data = &self.data[byte_offset as usize..];
                        let (_remains, sample) = be_f64::<_, Error<_>>(data).finish().unwrap();
                        return Some(sample as f32); // TODO f32にダウンキャストするべきなのか検討
                    }
                    _ => {
                        return None;
                    }
                }
            }
            AudioFormat::ImaAdpcm => {
                todo!();
            }
        }
    }
}

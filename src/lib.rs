use nom::error::Error;
use nom::number::complete::{le_f32, le_f64, le_i16, le_i24, le_i32};
use nom::Finish;
use nom::{multi::many1, IResult};

mod wav;

#[derive(Debug, Default)]
enum AudioFormat {
    #[default]
    Unknown,
    LinearPcmLe,
    LinearPcmBe,
    IeeeFloat,
    ALaw,
    MuLaw,
    ImaAdpcm,
}

#[derive(Default, Debug)]
pub struct PcmSpecs {
    audio_format: AudioFormat,
    num_channels: u16,
    sample_rate: u32,
    bit_depth: u16,
}

/// dataとwavとcは将来的にどれかに集約される。
/// read_bytes()のinputの型が決定したら再検討する。
#[derive(Default)]
pub struct PcmReader<'a> {
    specs: PcmSpecs,
    data: &'a [u8],
}

impl<'a> PcmReader<'a> {
    fn parse_aiff(&mut self, input: &'a [u8]) -> IResult<&[u8], &[u8]> {
        todo!(); // Ok((input, input))
    }

    fn parse_wav(&mut self, input: &'a [u8]) -> IResult<&[u8], &[u8]> {
        //many1はallocが実装されていないと使えない。no_stdで使うなら逐次的に実行するべき。
        let (_input, v) = many1(wav::parse_chunk)(input)?;

        for e in v {
            match e.id {
                wav::ChunkId::Fmt => {
                    println!("fmt");
                    let (_, spec) = wav::parse_fmt(e.data)?;
                    println!("{:?}", spec);
                    self.specs = spec;
                }
                wav::ChunkId::Data => {
                    println!("Data");
                    self.data = e.data;
                }
                wav::ChunkId::Fact => println!("fact"),
                wav::ChunkId::IDv3 => println!("IDv3"),
                wav::ChunkId::JUNK => println!("JUNK"),
                wav::ChunkId::LIST => println!("LIST"),
                wav::ChunkId::PEAK => println!("PEAK"),
                wav::ChunkId::Unknown => println!("Unknown"),
            }
        }
        return Ok((&[], &[]));
    }

    /// WAVのByte配列をパースし、再生できるように準備する。
    /// inputを
    /// * Arc<&[u8]>
    /// * Arc<[u8; size]>
    /// * &[u8]
    /// のどれにするか検討中。
    /// PcmReaderがいつ破棄されるかは再生時にしか決められない場合があるのでArcを使うべきだと思うが、スライスだと結局ライフタイムの問題がある。
    /// 少なくともinputとPcmReaderのlifetimeの長さがinput>PcmReaderであればよい。    
    /// http://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/book/second-edition/ch19-02-advanced-lifetimes.html
    /// また、配列だと長さがコンパイル時に決められない。ジェネリクスで書くのか、どう書くのがRust的に良いかを探っている。
    /// これをPcmReaderのnew()相当の初期化関数とするべきかもしれない。
    pub fn read_bytes(input: &'a [u8]) -> Self {
        let file_length = input.len();

        let mut reader: PcmReader = Default::default();

        //WAVの場合
        if let Ok((input, riff)) = wav::parse_riff_header(input) {
            println!("Riff length in bytes: {}", riff.size);
            assert_eq!(riff.id, wav::RiffIdentifier::Wave);
            assert_eq!((file_length - 8) as u32, riff.size);
            if let Ok((_, _)) = reader.parse_wav(input) {
                return reader;
            }
        };

        //AIFFの場合
        todo!();

        //WAVでもAIFFでもなかった場合
        panic!();
    }

    /// DATAチャンクを読んでサンプルを読みだす    
    /// フォーマットに関わらず+/-1の範囲に正規化された数を返す
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
                todo!();
            }
            AudioFormat::IeeeFloat => {
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
            AudioFormat::ALaw => {
                todo!();
            }
            AudioFormat::MuLaw => {
                todo!();
            }
            AudioFormat::ImaAdpcm => {
                todo!();
            }
        }
    }
}

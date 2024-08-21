//! pacmog is a decoding library for PCM files for embedded environments.  
//!
//! Rust has an include_bytes! macro to embed the byte sequence in the program.   
//! Using it, PCM files can be embedded in firmware and used for playback.  
//!
//! # Examples
//!
//! Read a sample WAV file.
//! ```
//! use pacmog::PcmReaderBuilder;
//!
//! let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");                        
//! let reader = PcmReaderBuilder::new(wav).build().unwrap();
//! let specs = reader.get_pcm_specs();
//! let num_samples = specs.num_samples;
//! let num_channels = specs.num_channels as u32;
//!
//! println!("PCM info: {:?}", specs);
//!
//! for sample in 0..num_samples {
//!     for channel in 0..num_channels {
//!         let sample_value = reader.read_sample(channel, sample).unwrap();
//!         println!("{}", sample_value);
//!     }
//! }
//! ```
#![cfg_attr(not(test), no_std)]

use heapless::Vec;
use nom::number::complete::{
    be_f32, be_f64, be_i16, be_i24, be_i32, le_f32, le_f64, le_i16, le_i24, le_i32,
};
use nom::Finish;
use nom::{multi::fold_many1, IResult};

mod aiff;
pub mod imaadpcm;
mod wav;

const MAX_NUM_CHUNKS: usize = 16;

/// Error type for LinearPCM
#[derive(Debug, thiserror::Error)]
pub enum LinearPcmError {
    #[error("Unsupported bit-depth")]
    UnsupportedBitDepth,
    #[error("Unsupported audio format")]
    UnsupportedAudioFormat,
    #[error("Invalid channel")]
    InvalidChannel,
    #[error("Invalid sample")]
    InvalidSample,
    #[error("Invalid output buffer length")]
    InvalidOutputBufferLength,
    #[error("Finished playing")]
    FinishPlaying,
    #[error("RIFF or AIFF header size mismatch")]
    HeaderSizeMismatch,
}

/// Audio format
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum AudioFormat {
    /// Unknown format
    #[default]
    Unknown,
    /// Linear PCM little endian    
    LinearPcmLe,
    /// Linear PCM big endian
    LinearPcmBe,
    /// IEEE float big endian
    IeeeFloatLe,
    /// IEEE float little endian
    IeeeFloatBe,
    /// IMA-ADPCM little endian
    ImaAdpcmLe,
}

/// Basic information on the PCM file.
#[derive(Default, Debug, Clone)]
pub struct PcmSpecs {
    /// Audio format.
    pub audio_format: AudioFormat,
    /// Number of channels.
    pub num_channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Bit depth.
    pub bit_depth: u16,
    /// Number of samples per channel.
    pub num_samples: u32,
    /// IMA-ADPCM only. Number of bytes per block of IMA-ADPCM.
    pub(crate) ima_adpcm_num_block_align: Option<u16>,
    /// IMA-ADPCM only. Number of samples per block of IMA-ADPCM.
    pub(crate) ima_adpcm_num_samples_per_block: Option<u16>,
}

/// Reads low level information and Data chunks from the PCM file.
#[derive(Default)]
pub struct PcmReader<'a> {
    pub(crate) specs: PcmSpecs,
    pub(crate) data: &'a [u8],
}

impl<'a> PcmReader<'a> {
    pub fn new(input: &'a [u8]) -> Result<Self, LinearPcmError> {
        let file_length = input.len();
        let mut reader = PcmReader {
            data: &[],
            specs: PcmSpecs::default(),
        };

        // Parse WAVE format
        if let Ok((input, riff)) = wav::parse_riff_header(input) {
            if (file_length - 8) != riff.size as usize {
                return Err(LinearPcmError::HeaderSizeMismatch);
            }

            if let Ok((_, _)) = reader.parse_wav(input) {
                return Ok(reader);
            }
        }

        // Parse AIFF format
        if let Ok((input, aiff)) = aiff::parse_aiff_header(input) {
            if (file_length - 8) != aiff.size as usize {
                return Err(LinearPcmError::HeaderSizeMismatch);
            }

            if let Ok((_, _)) = reader.parse_aiff(input) {
                return Ok(reader);
            }
        }

        Err(LinearPcmError::UnsupportedAudioFormat)
    }
    fn parse_aiff(&mut self, input: &'a [u8]) -> IResult<&[u8], &[u8]> {
        let (input, v) = fold_many1(
            aiff::parse_chunk,
            Vec::<aiff::Chunk, MAX_NUM_CHUNKS>::new,
            |mut chunk_array: Vec<aiff::Chunk, MAX_NUM_CHUNKS>, item| {
                chunk_array.push(item).unwrap();
                chunk_array
            },
        )(input)?;

        for chunk in v {
            match chunk.id {
                aiff::ChunkId::Common => {
                    let (_, spec) = aiff::parse_comm(chunk.data)?;
                    self.specs = spec;
                }
                aiff::ChunkId::SoundData => {
                    let (data, ssnd_block_info) = aiff::parse_ssnd(chunk.data)?;
                    // offset and block_size are typically 0. Therefore, this only supports files where they are set to 0.
                    if ssnd_block_info.offset != 0 || ssnd_block_info.block_size != 0 {
                        return Err(nom::Err::Error(nom::error::Error::new(
                            input,
                            nom::error::ErrorKind::Verify,
                        )));
                    }
                    self.data = data;
                }
                aiff::ChunkId::FormatVersion => {}
                aiff::ChunkId::Marker => {}
                aiff::ChunkId::Instrument => {}
                aiff::ChunkId::Midi => {}
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
        Ok((input, &[]))
    }

    fn parse_wav(&mut self, input: &'a [u8]) -> IResult<&[u8], &[u8]> {
        let (input, v) = fold_many1(
            wav::parse_chunk,
            Vec::<wav::Chunk, MAX_NUM_CHUNKS>::new,
            |mut chunk_array: Vec<wav::Chunk, MAX_NUM_CHUNKS>, item| {
                chunk_array.push(item).unwrap();
                chunk_array
            },
        )(input)?;

        for chunk in v {
            match chunk.id {
                wav::ChunkId::Fmt => {
                    let (_, spec) = wav::parse_fmt(chunk.data)?;
                    self.specs.num_channels = spec.num_channels;
                    self.specs.sample_rate = spec.sample_rate;
                    self.specs.audio_format = spec.audio_format;
                    self.specs.bit_depth = spec.bit_depth;
                    if self.specs.audio_format == AudioFormat::ImaAdpcmLe {
                        self.specs.ima_adpcm_num_block_align = spec.ima_adpcm_num_block_align;
                        self.specs.ima_adpcm_num_samples_per_block =
                            spec.ima_adpcm_num_samples_per_block;
                    }
                }
                wav::ChunkId::Data => {
                    self.data = chunk.data;
                }
                wav::ChunkId::Fact => {}
                wav::ChunkId::IDv3 => {}
                wav::ChunkId::Junk => {}
                wav::ChunkId::List => {}
                wav::ChunkId::Peak => {}
                wav::ChunkId::Unknown => {}
            }
        }

        match self.specs.audio_format {
            AudioFormat::ImaAdpcmLe => {
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
        Ok((input, &[]))
    }

    /// Returns basic information about the PCM file.
    #[must_use]
    pub fn get_pcm_specs(&self) -> PcmSpecs {
        self.specs.clone()
    }

    /// Returns the value of a sample at an arbitrary position.  
    /// Returns a normalized value in the range +/-1.0 regardless of AudioFormat.  
    pub fn read_sample(&self, channel: u32, sample: u32) -> Result<f32, LinearPcmError> {
        if channel >= self.specs.num_channels as u32 {
            return Err(LinearPcmError::InvalidChannel);
        }

        if sample >= self.specs.num_samples {
            return Err(LinearPcmError::InvalidSample);
        }

        let byte_depth = self.specs.bit_depth as u32 / 8u32;
        let byte_offset = ((byte_depth * sample * self.specs.num_channels as u32)
            + (byte_depth * channel)) as usize;
        let data = &self.data[byte_offset..];
        decode_sample(&self.specs, data)
    }
}

/// Decode a sample from a byte array.
/// Returns a normalized value in the range +/-1.0 regardless of AudioFormat.
/// TODO return not only f32 but also Q15, Q23, f64, etc.
/// Or make it possible to select f32 or f64.
/// It may be better to use a function like read_raw_sample() to get fixed-point numbers.
fn decode_sample(specs: &PcmSpecs, data: &[u8]) -> Result<f32, LinearPcmError> {
    match specs.audio_format {
        AudioFormat::Unknown => Err(LinearPcmError::UnsupportedAudioFormat),
        AudioFormat::LinearPcmLe => {
            match specs.bit_depth {
                16 => {
                    const MAX: u32 = 2u32.pow(15); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) =
                        le_i16::<_, nom::error::Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    Ok(sample)
                }
                24 => {
                    const MAX: u32 = 2u32.pow(23); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) =
                        le_i24::<_, nom::error::Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    Ok(sample)
                }
                32 => {
                    const MAX: u32 = 2u32.pow(31); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) =
                        le_i32::<_, nom::error::Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    Ok(sample)
                }
                _ => Err(LinearPcmError::UnsupportedBitDepth),
            }
        }
        AudioFormat::LinearPcmBe => {
            match specs.bit_depth {
                16 => {
                    const MAX: u32 = 2u32.pow(15); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) =
                        be_i16::<_, nom::error::Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    Ok(sample)
                }
                24 => {
                    const MAX: u32 = 2u32.pow(23); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) =
                        be_i24::<_, nom::error::Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    Ok(sample)
                }
                32 => {
                    const MAX: u32 = 2u32.pow(31); //normalize factor: 2^(BitDepth-1)
                    let (_remains, sample) =
                        be_i32::<_, nom::error::Error<_>>(data).finish().unwrap();
                    let sample = sample as f32 / MAX as f32;
                    Ok(sample)
                }
                _ => Err(LinearPcmError::UnsupportedBitDepth),
            }
        }
        AudioFormat::IeeeFloatLe => {
            match specs.bit_depth {
                32 => {
                    //32bit float
                    let (_remains, sample) =
                        le_f32::<_, nom::error::Error<_>>(data).finish().unwrap();
                    Ok(sample)
                }
                64 => {
                    //64bit float
                    let (_remains, sample) =
                        le_f64::<_, nom::error::Error<_>>(data).finish().unwrap();
                    Ok(sample as f32) // TODO f32にダウンキャストするべきなのか検討
                }
                _ => Err(LinearPcmError::UnsupportedBitDepth),
            }
        }
        AudioFormat::IeeeFloatBe => {
            match specs.bit_depth {
                32 => {
                    //32bit float
                    let (_remains, sample) =
                        be_f32::<_, nom::error::Error<_>>(data).finish().unwrap();
                    Ok(sample)
                }
                64 => {
                    //64bit float
                    let (_remains, sample) =
                        be_f64::<_, nom::error::Error<_>>(data).finish().unwrap();
                    Ok(sample as f32) // TODO f32にダウンキャストするべきなのか検討
                }
                _ => Err(LinearPcmError::UnsupportedBitDepth),
            }
        }
        AudioFormat::ImaAdpcmLe => Err(LinearPcmError::UnsupportedAudioFormat),
    }
}

/// High level of organized players for LinearPCM (WAVE or AIFF) file.
#[derive(Default)]
pub struct PcmPlayer<'a> {
    /// A reader to access basic information about the PCM file.
    pub reader: PcmReader<'a>,
    reading_data: &'a [u8],
    loop_playing: bool,
}

impl<'a> PcmPlayer<'a> {
    /// * 'input' - PCM data byte array
    pub fn new(input: &'a [u8]) -> Self {
        //TODO error handling
        let reader = PcmReader::new(input).unwrap();

        let mut player = PcmPlayer {
            reader,
            reading_data: &[],
            loop_playing: false,
        };
        player.set_position(0);
        player
    }

    /// Move the playback position to the desired position.    
    pub fn set_position(&mut self, sample: u32) {
        let byte_depth = self.reader.specs.bit_depth as u32 / 8u32;
        let byte_offset = (byte_depth * sample * self.reader.specs.num_channels as u32) as usize;
        self.reading_data = &self.reader.data[byte_offset..];
    }

    /// Enable loop playback.
    /// true: Enable loop playback
    /// false: Disable loop playback
    pub fn set_loop_playing(&mut self, en: bool) {
        self.loop_playing = en;
    }

    /// Return samples value of the next frame.
    /// * ‘out’ - Output buffer which the sample values are written. Number of elements must be equal to or greater than the number of channels in the PCM file.
    pub fn get_next_frame(&mut self, out: &mut [f32]) -> Result<(), LinearPcmError> {
        let byte_depth = self.reader.specs.bit_depth / 8;

        if out.len() < self.reader.specs.num_channels as usize {
            return Err(LinearPcmError::InvalidOutputBufferLength);
        }

        if self.reading_data.is_empty() {
            if self.loop_playing {
                self.set_position(0);
            } else {
                return Err(LinearPcmError::FinishPlaying);
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

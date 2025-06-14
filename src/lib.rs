//! pacmog is a decoding library for PCM files for embedded environments.  
//!
//! Rust has an include_bytes! macro to embed the byte sequence in the program.   
//! Using it, PCM files can be embedded in firmware and used for playback.  
//!
//! # Examples
//!
//! Read a sample WAV file.
//! ```
//! use pacmog::PcmReader;
//! # use pacmog::PcmReaderError;
//!
//! # fn main() -> Result<(), pacmog::PcmReaderError> {
//! let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
//! let mut input = &wav[..];
//! let reader = PcmReader::new(&mut input)?;
//! let specs = reader.get_pcm_specs();
//! let num_samples = specs.num_samples;
//! let num_channels = specs.num_channels;
//!
//! println!("PCM info: {:?}", specs);
//!
//! for sample in 0..num_samples {
//!     for channel in 0..num_channels {
//!         let sample_value: f32 = reader.read_sample(channel, sample)?;
//!         println!("{}", sample_value);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![cfg_attr(not(test), no_std)]

use heapless::Vec;
use num_traits::float::Float;
use winnow::binary::{
    be_f32, be_f64, be_i16, be_i24, be_i32, le_f32, le_f64, le_i16, le_i24, le_i32,
};
use winnow::{ModalResult, Parser};

mod aiff;
pub mod imaadpcm;
mod wav;

const MAX_NUM_CHUNKS: usize = 16;

/// Error type for LinearPCM
#[derive(Debug, thiserror::Error)]
pub enum PcmReaderError {
    #[error("Unsupported bit-depth")]
    UnsupportedBitDepth,
    #[error("Unsupported audio format")]
    UnsupportedAudioFormat,
    #[error("Invalid channel")]
    InvalidChannel,
    #[error("Invalid sample")]
    InvalidSample,
    #[error("RIFF or AIFF header size mismatch")]
    HeaderSizeMismatch,
    #[error("fmt parse error")]
    FmtParseError,
    #[error("Header parse error")]
    HeaderParseError,
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
///
/// # Examples
///
/// Read a sample WAV file.
/// ```
/// use pacmog::PcmReader;
/// # use pacmog::PcmReaderError;
///
/// # fn main() -> Result<(), pacmog::PcmReaderError> {
/// let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
/// let mut input = &wav[..];
/// let reader = PcmReader::new(&mut input)?;
/// let specs = reader.get_pcm_specs();
/// let num_samples = specs.num_samples;
/// let num_channels = specs.num_channels;
///
/// println!("PCM info: {:?}", specs);
///
/// for sample in 0..num_samples {
///     for channel in 0..num_channels {
///         let sample_value: f32 = reader.read_sample(channel, sample)?;
///         println!("{}", sample_value);
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct PcmReader<'a> {
    pub(crate) specs: PcmSpecs,
    pub(crate) data: &'a [u8],
}

impl<'a> PcmReader<'a> {
    /// Create a new PcmReader instance.
    ///
    /// # Arguments
    ///
    /// * 'input' - PCM data byte array
    ///
    /// # Errors
    ///
    /// If the input data is invalid or the PCM specs are not supported, an error will be returned.
    pub fn new(input: &mut &'a [u8]) -> Result<Self, PcmReaderError> {
        let mut reader = PcmReader {
            data: &[],
            specs: PcmSpecs::default(),
        };
        reader.reload(input)?;
        Ok(reader)
    }

    /// Reload a new PCM byte array.
    ///
    /// # Arguments
    ///
    /// * 'input' - PCM data byte array
    ///
    /// # Errors
    ///
    /// If the input data is invalid or the PCM specs are not supported, an error will be returned.
    pub fn reload(&mut self, input: &mut &'a [u8]) -> Result<(), PcmReaderError> {
        let file_length = input.len();
        self.data = &[];
        self.specs = PcmSpecs::default();

        // Parse WAVE format
        // inputを消費しないようにparse_nextではなくparse_peekを使用している
        if let Ok((_, riff)) = wav::parse_riff_header.parse_peek(input) {
            if file_length - 8 != riff.size as usize {
                return Err(PcmReaderError::HeaderSizeMismatch);
            }

            return self.parse_wav(input);
        };

        // Parse AIFF format
        // inputを消費しないようにparse_nextではなくparse_peekを使用している
        if let Ok((_, aiff)) = aiff::parse_aiff_header.parse_peek(input) {
            if (file_length - 8) != aiff.size as usize {
                return Err(PcmReaderError::HeaderSizeMismatch);
            }

            return self.parse_aiff(input);
        };

        Err(PcmReaderError::UnsupportedAudioFormat)
    }

    /// Parse AIFF format.
    ///
    /// # Arguments
    ///
    /// * `input` - PCM data byte array
    fn parse_aiff(&mut self, input: &mut &'a [u8]) -> Result<(), PcmReaderError> {
        // Parse AIFF header
        let Ok(_) = aiff::parse_aiff_header.parse_next(input) else {
            return Err(PcmReaderError::HeaderParseError);
        };

        let mut chunk_vec = Vec::<aiff::Chunk, MAX_NUM_CHUNKS>::new();

        // Parse chunks
        while let Ok(chunk) = aiff::parse_chunk.parse_next(input) {
            chunk_vec.push(chunk).unwrap();
        }

        for mut chunk in chunk_vec {
            match chunk.id {
                aiff::ChunkId::Common => {
                    let Ok(spec) = aiff::parse_comm.parse_next(&mut chunk.data) else {
                        return Err(PcmReaderError::UnsupportedAudioFormat);
                    };
                    self.specs = spec;
                }
                aiff::ChunkId::SoundData => {
                    let Ok(_ssnd_block_info) = aiff::parse_ssnd.parse_next(&mut chunk.data) else {
                        return Err(PcmReaderError::UnsupportedAudioFormat);
                    };
                    self.data = chunk.data;
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
        Ok(())
    }

    /// Parse WAVE format.
    ///
    /// # Arguments
    ///
    /// * `input` - PCM data byte array
    fn parse_wav(&mut self, input: &mut &'a [u8]) -> Result<(), PcmReaderError> {
        // Parse RIFF header
        let Ok(_) = wav::parse_riff_header.parse_next(input) else {
            return Err(PcmReaderError::HeaderParseError);
        };

        let mut chunk_vec = Vec::<wav::Chunk, MAX_NUM_CHUNKS>::new();

        // Parse chunks
        while let Ok(chunk) = wav::parse_chunk.parse_next(input) {
            chunk_vec.push(chunk).unwrap();
        }

        for mut chunk in chunk_vec {
            match chunk.id {
                wav::ChunkId::Fmt => {
                    let Ok(spec) = wav::parse_fmt.parse_next(&mut chunk.data) else {
                        return Err(PcmReaderError::FmtParseError);
                    };
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
        Ok(())
    }

    /// Returns basic information about the PCM file.
    #[must_use]
    pub fn get_pcm_specs(&self) -> PcmSpecs {
        self.specs.clone()
    }

    /// Read the sample at an arbitrary position.
    ///
    /// # Arguments
    ///
    /// * 'channel' - Channel number (0-indexed)
    /// * 'sample' - Sample number (0-indexed)
    ///
    /// # Returns
    ///
    /// Returns a normalized value in the range +/-1.0 regardless of AudioFormat.
    pub fn read_sample<T: Float>(&self, channel: u16, sample: u32) -> Result<T, PcmReaderError> {
        let num_channels = self.specs.num_channels;
        if channel >= num_channels {
            return Err(PcmReaderError::InvalidChannel);
        }

        if sample >= self.specs.num_samples {
            return Err(PcmReaderError::InvalidSample);
        }

        let byte_depth = self.specs.bit_depth / 8u16;
        let byte_offset = ((byte_depth as u32 * sample * num_channels as u32)
            + (byte_depth * channel) as u32) as usize;
        let mut data = &self.data[byte_offset..];
        decode_sample(&self.specs, &mut data)
    }
}

/// Decode a sample from a byte array.
///
/// # Arguments
///
/// * 'specs' - PCM file specifications
/// * 'data' - Byte array containing the sample data
///
/// # Returns
///
/// Returns a normalized value in the range +/-1.0 regardless of AudioFormat.
fn decode_sample<T: Float>(specs: &PcmSpecs, data: &mut &[u8]) -> Result<T, PcmReaderError> {
    match specs.audio_format {
        AudioFormat::Unknown => Err(PcmReaderError::UnsupportedAudioFormat),
        AudioFormat::LinearPcmLe => match specs.bit_depth {
            16 => {
                const MAX: u32 = 2u32.pow(15);
                let res: ModalResult<i16> = le_i16.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap() / T::from(MAX).unwrap())
            }
            24 => {
                const MAX: u32 = 2u32.pow(23);
                let res: ModalResult<i32> = le_i24.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap() / T::from(MAX).unwrap())
            }
            32 => {
                const MAX: u32 = 2u32.pow(31);
                let res: ModalResult<i32> = le_i32.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap() / T::from(MAX).unwrap())
            }
            _ => Err(PcmReaderError::UnsupportedBitDepth),
        },
        AudioFormat::LinearPcmBe => match specs.bit_depth {
            16 => {
                const MAX: u32 = 2u32.pow(15);
                let res: ModalResult<i16> = be_i16.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap() / T::from(MAX).unwrap())
            }
            24 => {
                const MAX: u32 = 2u32.pow(23);
                let res: ModalResult<i32> = be_i24.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap() / T::from(MAX).unwrap())
            }
            32 => {
                const MAX: u32 = 2u32.pow(31);
                let res: ModalResult<i32> = be_i32.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap() / T::from(MAX).unwrap())
            }
            _ => Err(PcmReaderError::UnsupportedBitDepth),
        },
        AudioFormat::IeeeFloatLe => match specs.bit_depth {
            32 => {
                let res: ModalResult<f32> = le_f32.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap())
            }
            64 => {
                let res: ModalResult<f64> = le_f64.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap())
            }
            _ => Err(PcmReaderError::UnsupportedBitDepth),
        },
        AudioFormat::IeeeFloatBe => match specs.bit_depth {
            32 => {
                let res: ModalResult<f32> = be_f32.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap())
            }
            64 => {
                let res: ModalResult<f64> = be_f64.parse_next(data);
                let Ok(sample) = res else {
                    return Err(PcmReaderError::InvalidSample);
                };
                Ok(T::from(sample).unwrap())
            }
            _ => Err(PcmReaderError::UnsupportedBitDepth),
        },
        AudioFormat::ImaAdpcmLe => Err(PcmReaderError::UnsupportedAudioFormat),
    }
}

/// Error type for PcmPlayer
#[derive(Debug, thiserror::Error)]
pub enum PcmPlayerError {
    #[error("Output buffer too short")]
    OutputBufferTooShort,
    #[error("Invalid position")]
    InvalidPosition,
    #[error("Finish playing")]
    FinishPlaying,
}

/// High level of organized players for LinearPCM (WAVE or AIFF) file.
///
/// # Examples
///
/// Play a sample WAV file.
/// ```
/// use pacmog::{PcmPlayer, PcmReader};
///
/// # fn main() -> anyhow::Result<()> {
/// let wav = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_16.wav");
/// let mut input = &wav[..];
/// let reader = PcmReader::new(&mut input)?;
/// let mut player = PcmPlayer::new(reader);
/// let specs = player.reader.get_pcm_specs();
/// let num_samples = specs.num_samples;
///
/// player.set_loop_playing(true);
/// let mut buffer: [f32; 2] = [0.0f32; 2];
/// let buf = buffer.as_mut_slice();
///
/// for sample in 0..num_samples {
///    player.get_next_frame(buf)?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct PcmPlayer<'a> {
    /// A reader to access basic information about the PCM file.
    pub reader: PcmReader<'a>,
    playback_position: u32,
    loop_playing: bool,
}

impl<'a> PcmPlayer<'a> {
    /// * 'input' - PCM data byte array
    pub fn new(reader: PcmReader<'a>) -> Self {
        PcmPlayer {
            reader,
            playback_position: 0,
            loop_playing: false,
        }
    }

    /// Move the playback position to the desired position.
    /// * 'sample' - Playback position in samples.
    pub fn set_position(&mut self, sample: u32) -> Result<(), PcmPlayerError> {
        if self.reader.specs.num_samples <= sample {
            return Err(PcmPlayerError::InvalidPosition);
        }
        self.playback_position = sample;
        Ok(())
    }

    /// Enable loop playback.
    ///
    /// true: Enable loop playback
    /// false: Disable loop playback
    pub fn set_loop_playing(&mut self, en: bool) {
        self.loop_playing = en;
    }

    /// Return samples value of the next frame.
    ///
    /// # Arguments
    ///
    /// * ‘out’ - Output buffer which the sample values are written. Number of elements must be equal to or greater than the number of channels in the PCM file.
    ///
    /// # Errors
    ///
    /// If the output buffer is too short, an error will be returned.
    /// If the loop playback is disabled and the end of the file is reached, an error will be returned.
    pub fn get_next_frame<T: Float>(&mut self, out: &mut [T]) -> Result<(), PcmPlayerError> {
        if out.len() < self.reader.specs.num_channels as usize {
            return Err(PcmPlayerError::OutputBufferTooShort);
        }

        let num_samples = self.reader.specs.num_samples;
        if self.playback_position >= num_samples {
            if self.loop_playing {
                self.set_position(0)?;
            } else {
                return Err(PcmPlayerError::FinishPlaying);
            }
        }

        let num_chennels = self.reader.specs.num_channels;
        for ch in 0..num_chennels {
            let Ok(sample) = self.reader.read_sample(ch, self.playback_position) else {
                return Err(PcmPlayerError::InvalidPosition);
            };
            out[ch as usize] = sample;
        }

        // Update the playback position.
        self.playback_position += 1;

        Ok(())
    }
}

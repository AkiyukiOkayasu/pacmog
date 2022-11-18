use core::convert::TryInto;
use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_i16, be_i32, be_u32};
use nom::IResult;

use crate::{AudioFormat, PcmSpecs};

/// ckID chunkの種類
#[derive(Debug, PartialEq, Default)]
pub(super) enum ChunkId {
    Common,              // b"COMM" Common
    SoundData,           // b"SSND" Sound data
    Marker,              // b"MARK" optional
    FormatVersion,       // b"FVER" optional AIFF-C only
    Instrument,          // b"INST"
    MIDI,                // b"MIDI"
    AudioRecording,      // b"AESD"
    ApplicationSpecific, // b"APPL"
    Comment,             // b""COMT"
    Name,                // b"NAME" text chunk
    Author,              // b"AUTH" text chunk
    Copyright,           // b"(c) " text chunk
    Annotation,          // b"ANNO" text chunk
    #[default]
    Unknown,
}

impl TryFrom<&[u8]> for ChunkId {
    type Error = ();

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() != 4 {
            return Err(());
        }

        match v {
            b"COMM" => Ok(ChunkId::Common),
            b"SSND" => Ok(ChunkId::SoundData),
            b"FVER" => Ok(ChunkId::FormatVersion),
            b"MARK" => Ok(ChunkId::Marker),
            b"INST" => Ok(ChunkId::Instrument),
            b"MIDI" => Ok(ChunkId::MIDI),
            b"AESD" => Ok(ChunkId::AudioRecording),
            b"APPL" => Ok(ChunkId::ApplicationSpecific),
            b"COMT" => Ok(ChunkId::Comment),
            b"NAME" => Ok(ChunkId::Name),
            b"AUTH" => Ok(ChunkId::Author),
            b"(c) " => Ok(ChunkId::Copyright),
            b"ANNO" => Ok(ChunkId::Annotation),
            _ => Ok(ChunkId::Unknown),
        }
    }
}

#[derive(Debug)]
enum CompressionTypeId {
    None,
    Sowt,
    FL32,
    FL64,
}

impl TryFrom<&[u8]> for CompressionTypeId {
    type Error = ();

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() != 4 {
            return Err(());
        }

        match v {
            b"NONE" => Ok(CompressionTypeId::None),
            b"sowt" => Ok(CompressionTypeId::Sowt),
            b"fl32" => Ok(CompressionTypeId::FL32),
            b"FL32" => Ok(CompressionTypeId::FL32),
            b"fl64" => Ok(CompressionTypeId::FL64),
            b"FL64" => Ok(CompressionTypeId::FL64),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Chunk<'a> {
    pub id: ChunkId,
    pub size: u32,
    pub data: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub(super) enum AiffIdentifier {
    Aiff,  //b"AIFF"
    AiffC, //b"AIFC" AIFF-C
}

impl TryFrom<&[u8]> for AiffIdentifier {
    type Error = ();

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() != 4 {
            return Err(());
        }

        match v {
            b"AIFF" => Ok(AiffIdentifier::Aiff),
            b"AIFC" => Ok(AiffIdentifier::AiffC),
            _ => Err(()),
        }
    }
}

/// AIFFチャンクの情報
///
/// * 'size' - ファイルサイズ(byte) - 8
/// * 'id' - RIFFの識別子 基本"WAVE"
#[derive(Debug)]
pub(super) struct AiffHeader {
    pub size: u32,
    pub id: AiffIdentifier,
}

/// SSNDチャンクのOffset, BlockSize
/// ほとんどの場合、offsetもblock_sizeも0になる
///
/// * 'offset' - ほとんどの場合0
/// * 'block_size' - ほとんどの場合0
#[derive(Debug)]
pub(super) struct SsndBlockInfo {
    pub offset: i32,
    pub block_size: i32,
}

/// ファイルがFORMから始まり、識別子がAIFFであることのチェック
///
pub(super) fn parse_aiff_header(input: &[u8]) -> IResult<&[u8], AiffHeader> {
    let (input, _) = tag(b"FORM")(input)?;
    let (input, size) = be_u32(input)?;
    let (input, id) = take(4usize)(input)?;
    let id: AiffIdentifier = id.try_into().unwrap();
    Ok((input, AiffHeader { size, id }))
}

pub(super) fn parse_chunk(input: &[u8]) -> IResult<&[u8], Chunk> {
    let (input, id) = take(4usize)(input)?;
    let id: ChunkId = id.try_into().unwrap();
    let (input, size) = be_u32(input)?;
    let (input, data) = take(size)(input)?;

    Ok((input, Chunk { id, size, data }))
}

/// COMMONチャンクのパース
pub(super) fn parse_comm(input: &[u8]) -> IResult<&[u8], PcmSpecs> {
    let mut audio_format: AudioFormat = AudioFormat::LinearPcmBe;

    let (input, num_channels) = be_i16(input)?;
    let num_channels = num_channels as u16;
    let (input, num_sample_frames) = be_u32(input)?;
    let (input, bit_depth) = be_i16(input)?;
    let bit_depth = bit_depth as u16;
    let (input, sample_rate) = take(10usize)(input)?;
    let sample_rate = extended2double(sample_rate) as u32;

    if input.len() >= 4 {
        //AIFF-C parameters
        let (_input, compression_type_id) = take(4usize)(input)?;
        let compression_type_id: CompressionTypeId = compression_type_id.try_into().unwrap();
        audio_format = match compression_type_id {
            CompressionTypeId::None => AudioFormat::LinearPcmBe,
            CompressionTypeId::Sowt => AudioFormat::LinearPcmLe,
            CompressionTypeId::FL32 => AudioFormat::IeeeFloatBe,
            CompressionTypeId::FL64 => AudioFormat::IeeeFloatBe,
        }
    }

    Ok((
        input,
        PcmSpecs {
            audio_format,
            num_channels,
            sample_rate,
            bit_depth,
        },
    ))
}

pub(super) fn parse_ssnd(input: &[u8]) -> IResult<&[u8], SsndBlockInfo> {
    let (input, offset) = be_i32(input)?;
    let (input, block_size) = be_i32(input)?;
    Ok((input, SsndBlockInfo { offset, block_size }))
}

/// 80 bit floating point value according to the IEEE-754 specification and the Standard Apple Numeric Environment specification:
/// 1 bit sign, 15 bit exponent, 1 bit normalization indication, 63 bit mantissa
/// https://stackoverflow.com/a/3949358
fn extended2double(buffer: &[u8]) -> f64 {
    assert!(buffer.len() >= 10);

    let sign = if (buffer[0] & 0x80) == 0x00 {
        1f64
    } else {
        -1f64
    };
    let exponent: u32 = ((buffer[0] as u32 & 0x7F) << 8) | buffer[1] as u32;
    let mut mantissa: u64 = ((buffer[2] as u64) << 56)
        | ((buffer[3] as u64) << 48)
        | ((buffer[4] as u64) << 40)
        | ((buffer[5] as u64) << 32)
        | ((buffer[6] as u64) << 24)
        | ((buffer[7] as u64) << 16)
        | ((buffer[8] as u64) << 8)
        | (buffer[9] as u64);

    //If the highest bit of the mantissa is set, then this is a normalized number.
    let normalize_correction = if (mantissa & 0x8000000000000000) != 0x00 {
        1f64
    } else {
        0f64
    };
    mantissa &= 0x7FFFFFFFFFFFFFFF;

    //value = (-1) ^ s * (normalizeCorrection + m / 2 ^ 63) * 2 ^ (e - 16383)
    sign * (normalize_correction + mantissa as f64 / 2f64.powf(63f64))
        * 2f64.powf(exponent as f64 - 16383f64)
}

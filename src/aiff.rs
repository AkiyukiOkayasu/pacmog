use nom::bytes::complete::{tag, tag_no_case, take};
use nom::error::Error;
use nom::number::complete::{be_i32, be_u32};
use nom::IResult;

use crate::{AudioFormat, PcmSpecs};

/// ckID chunkの種類
#[derive(Debug, PartialEq, Default)]
pub(crate) enum ChunkId {
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

#[derive(Debug, Default)]
pub(crate) struct Chunk<'a> {
    pub id: ChunkId,
    pub size: u32,
    pub data: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub(super) enum AiffIdentifier {
    Aiff, //b"AIFF"
    Aifc, //b"AIFC" AIFF-C
    Unknown,
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

/// ファイルがFORMから始まり、識別子がAIFFであることのチェック
///
pub(super) fn parse_aiff_header(input: &[u8]) -> IResult<&[u8], AiffHeader> {
    let (input, _) = tag(b"FORM")(input)?;
    let (input, size) = be_u32(input)?;
    let (input, id_str) = take(4usize)(input)?;

    let id: AiffIdentifier = match id_str {
        b"AIFF" => AiffIdentifier::Aiff,
        b"AIFC" => AiffIdentifier::Aifc,
        _ => AiffIdentifier::Unknown,
    };

    Ok((input, AiffHeader { size, id }))
}

pub(super) fn parse_chunk(input: &[u8]) -> IResult<&[u8], Chunk> {
    let (input, id) = take(4usize)(input)?;

    let id = match id {
        b"COMM" => ChunkId::Common,
        b"SSND" => ChunkId::SoundData,
        b"FVER" => ChunkId::FormatVersion,
        b"MARK" => ChunkId::Marker,
        b"INST" => ChunkId::Instrument,
        b"MIDI" => ChunkId::MIDI,
        b"AESD" => ChunkId::AudioRecording,
        b"APPL" => ChunkId::ApplicationSpecific,
        b"COMT" => ChunkId::Comment,
        b"NAME" => ChunkId::Name,
        b"AUTH" => ChunkId::Author,
        b"(c) " => ChunkId::Copyright,
        b"ANNO" => ChunkId::Annotation,
        _ => ChunkId::Unknown,
    };

    let (input, size) = be_u32(input)?;
    let (input, data) = take(size)(input)?;

    Ok((input, Chunk { id, size, data }))
}

/// COMMONチャンクのパース
pub(super) fn parse_comm(input: &[u8]) -> IResult<&[u8], PcmSpecs> {
    todo!();

    let (input, format) = le_u16(input)?;
    let wave_format_tag: WaveFormatTag = match format {
        0 => WaveFormatTag::Unknown,
        1 => WaveFormatTag::LinearPcm,
        3 => WaveFormatTag::IeeeFloat,
        6 => WaveFormatTag::ALaw,
        7 => WaveFormatTag::MuLaw,
        0x11 => WaveFormatTag::ImaAdpcm,
        _ => WaveFormatTag::Unknown,
    };

    let audio_format: AudioFormat = match wave_format_tag {
        WaveFormatTag::Unknown => AudioFormat::Unknown,
        WaveFormatTag::LinearPcm => AudioFormat::LinearPcmLe,
        WaveFormatTag::IeeeFloat => AudioFormat::IeeeFloat,
        WaveFormatTag::ALaw => AudioFormat::ALaw,
        WaveFormatTag::MuLaw => AudioFormat::MuLaw,
        WaveFormatTag::ImaAdpcm => AudioFormat::ImaAdpcm,
    };

    let (input, num_channels) = le_u16(input)?;
    let (input, sample_rate) = le_u32(input)?;
    let (input, _bytes_per_seconds) = le_u32(input)?;
    let (input, _block_size) = le_u16(input)?;
    let (input, bit_depth) = le_u16(input)?;

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

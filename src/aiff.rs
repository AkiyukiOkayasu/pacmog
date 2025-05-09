use crate::{AudioFormat, PcmSpecs};
use nom::branch::alt;
use nom::bytes::complete::{tag, take};
use nom::number::complete::{be_i16, be_i32, be_u32};
use nom::{IResult, Parser};

#[derive(thiserror::Error, Debug)]
enum AiffError {
    #[error("Buffer length must be exactly 10 bytes")]
    InvalidBufferLength,
}

/// AiffErrorをnom::Errに変換する
/// 変換時に情報が失われてしまう。とりあえずnom::error::ErrorKind::Failとしたが、IResultを使わずに実装できるか検討したい
impl From<AiffError> for nom::Err<nom::error::Error<&[u8]>> {
    fn from(_err: AiffError) -> Self {
        nom::Err::Error(nom::error::Error::new(&[], nom::error::ErrorKind::Fail))
    }
}

/// ckID chunkの種類
#[derive(Debug, PartialEq, Default)]
pub(super) enum ChunkId {
    Common,              // b"COMM" Common
    SoundData,           // b"SSND" Sound data
    Marker,              // b"MARK" optional
    FormatVersion,       // b"FVER" optional AIFF-C only
    Instrument,          // b"INST"
    Midi,                // b"MIDI"
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
            b"MIDI" => Ok(ChunkId::Midi),
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

#[derive(Debug, Default)]
pub(super) struct Chunk<'a> {
    pub id: ChunkId,
    #[allow(dead_code)]
    pub size: u32,
    pub data: &'a [u8],
}

/// AIFFチャンクの情報
/// * 'size' - ファイルサイズ(byte) - 8
pub(super) struct AiffHeader {
    pub size: u32,
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

/// ファイルがFORMから始まり、識別子がAIFFもしくはAIFF-Cであることのチェック
pub(super) fn parse_aiff_header(input: &[u8]) -> IResult<&[u8], AiffHeader> {
    let (input, _) = tag(b"FORM" as &[u8])(input)?;
    let (input, size) = be_u32(input)?;
    let (input, _id) = alt((tag(b"AIFF" as &[u8]), tag(b"AIFC" as &[u8]))).parse(input)?;
    Ok((input, AiffHeader { size }))
}

/// 先頭のチャンクを取得する
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
    let mut bit_depth = bit_depth as u16;
    let (input, sample_rate) = take(10usize)(input)?;
    let sample_rate = extended2double(sample_rate).map_err(nom::Err::from)? as u32;

    if input.len() >= 4 {
        //AIFF-C parameters
        let (_input, compression_type_id) = take(4usize)(input)?;
        let Ok((f, b)) = aifc_compression_type(compression_type_id) else {
            // Unknown compression type
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Fail,
            )));
        };
        audio_format = f;
        if let Some(b) = b {
            //bit-depthが指定されている場合は上書き
            bit_depth = b;
        }
    }

    Ok((
        input,
        PcmSpecs {
            audio_format,
            num_channels,
            sample_rate,
            bit_depth,
            num_samples: num_sample_frames,
            ..Default::default()
        },
    ))
}

// AIFF-CのCOMMONチャンクにのみ存在するcompressionTypeからEndian, bit-depthを決定する
fn aifc_compression_type(compression_type_id: &[u8]) -> Result<(AudioFormat, Option<u16>), ()> {
    let t = match compression_type_id {
        b"NONE" => (AudioFormat::LinearPcmBe, None),
        b"twos" => (AudioFormat::LinearPcmBe, Some(16)),
        b"sowt" => (AudioFormat::LinearPcmLe, Some(16)),
        b"fl32" => (AudioFormat::IeeeFloatBe, Some(32)),
        b"FL32" => (AudioFormat::IeeeFloatBe, Some(32)),
        b"fl64" => (AudioFormat::IeeeFloatBe, Some(64)),
        b"FL64" => (AudioFormat::IeeeFloatBe, Some(64)),
        b"in24" => (AudioFormat::LinearPcmBe, Some(24)),
        b"in32" => (AudioFormat::LinearPcmBe, Some(32)),
        b"42ni" => (AudioFormat::LinearPcmLe, Some(24)),
        b"23ni" => (AudioFormat::LinearPcmLe, Some(32)),
        _ => return Err(()), //Unknown compression type
    };
    Ok(t)
}

// SSNDチャンクのパース
pub(super) fn parse_ssnd(input: &[u8]) -> IResult<&[u8], SsndBlockInfo> {
    let (input, offset) = be_i32(input)?;
    let (input, block_size) = be_i32(input)?;
    Ok((input, SsndBlockInfo { offset, block_size }))
}

/// 80 bit floating point value according to the IEEE-754 specification and the Standard Apple Numeric Environment specification:
/// 1 bit sign, 15 bit exponent, 1 bit normalization indication, 63 bit mantissa
/// https://stackoverflow.com/a/3949358
fn extended2double(buffer: &[u8]) -> Result<f64, AiffError> {
    if buffer.len() != 10 {
        return Err(AiffError::InvalidBufferLength);
    }

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
    Ok(sign
        * (normalize_correction + mantissa as f64 / (1u64 << 63) as f64)
        * (1u64 << (exponent as i32 - 16383)) as f64)
}

#[cfg(test)]
mod tests {
    use super::{extended2double, ChunkId};
    use approx::assert_relative_eq;

    #[test]
    fn extended2double_test() {
        let array: [u8; 10] = [64, 14, 187, 128, 0, 0, 0, 0, 0, 0];
        assert_relative_eq!(extended2double(&array).unwrap(), 48000.0f64);
    }

    #[test]
    fn chunk_id_test() {
        let b = b"COMM";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Common);

        let b = b"SSND";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::SoundData);

        let b = b"MARK";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Marker);

        let b = b"FVER";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::FormatVersion);

        let b = b"INST";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Instrument);

        let b = b"MIDI";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Midi);

        let b = b"AESD";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::AudioRecording);

        let b = b"APPL";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::ApplicationSpecific);

        let b = b"COMT";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Comment);

        let b = b"NAME";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Name);

        let b = b"AUTH";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Author);

        let b = b"(c) ";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Copyright);

        let b = b"ANNO";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Annotation);

        let b = b"HOGE";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Unknown);

        let b = b"FOO";
        let e: Result<ChunkId, ()> = b.as_slice().try_into();
        assert_eq!(e, Err(()));
    }
}

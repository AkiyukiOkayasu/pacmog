use nom::bytes::complete::{tag, tag_no_case, take};
use nom::error::Error;
use nom::number::complete::{le_u16, le_u32};
use nom::IResult;

use crate::{AudioFormat, PcmSpecs};

/// chunkの種類
///
/// * "fmt " - 必須チャンク
/// * "fact" - optional
/// * "PEAK" - optional
/// * "data" - 必須チャンク
#[derive(Debug, PartialEq, Default)]
pub(crate) enum ChunkId {
    Fmt,  // b"fmt "
    Fact, // b"fact"
    PEAK, // b"PEAK"
    Data, // b"data"
    JUNK,
    LIST,
    IDv3,
    #[default]
    Unknown,
}

#[derive(Debug, Default)]
pub(crate) struct Chunk<'a> {
    pub id: ChunkId,
    pub size: u32,
    pub data: &'a [u8],
}

/// Waveの形式
/// LinearPCMとIEEE FloatとIMA ADPCMくらいしか使わないはず
/// https://github.com/tpn/winsdk-10/blob/9b69fd26ac0c7d0b83d378dba01080e93349c2ed/Include/10.0.14393.0/shared/mmreg.h#L2107-L2372
#[derive(Debug)]
pub(super) enum WaveFormatTag {
    Unknown = 0x00,   //0
    LinearPcm = 0x01, //1
    IeeeFloat = 0x03, //3
    ALaw = 0x06,      //6
    MuLaw = 0x07,     //7
    ImaAdpcm = 0x11,  //0x11 aka DVI ADPCM
}

#[derive(Debug, PartialEq)]
pub(super) enum RiffIdentifier {
    Wave, //b"WAVE"
    Avi,  //b"AVI "
    Unknown,
}

/// RIFFチャンクの情報
///
/// * 'size' - ファイルサイズ(byte) - 8
/// * 'id' - RIFFの識別子 基本"WAVE"
#[derive(Debug)]
pub(super) struct RiffHeader {
    pub size: u32,
    pub id: RiffIdentifier,
}

/// ファイルがRIFFから始まり、識別子がWAVEであることのチェック
pub(super) fn parse_riff_header(input: &[u8]) -> IResult<&[u8], RiffHeader> {
    let (input, _) = tag(b"RIFF")(input)?;
    let (input, size) = le_u32(input)?;
    let (input, id_str) = take(4usize)(input)?;

    let id: RiffIdentifier = match id_str {
        b"WAVE" => RiffIdentifier::Wave,
        b"AVI " => RiffIdentifier::Avi,
        _ => RiffIdentifier::Unknown,
    };

    Ok((input, RiffHeader { size, id }))
}

pub(super) fn parse_chunk(input: &[u8]) -> IResult<&[u8], Chunk> {
    let (input, chunk_id) = take(4usize)(input)?;

    let id = match chunk_id {
        b"fmt " => ChunkId::Fmt,
        b"fact" => ChunkId::Fact,
        b"PEAK" => ChunkId::PEAK,
        b"data" => ChunkId::Data,
        b"junk" => ChunkId::JUNK,
        b"JUNK" => ChunkId::JUNK,
        b"IDv3" => ChunkId::IDv3,
        b"LIST" => ChunkId::LIST,
        _ => ChunkId::Unknown,
    };

    let (input, size) = le_u32(input)?;
    let (input, data) = take(size)(input)?;
    Ok((input, Chunk { id, size, data }))
}

/// WAVはLittleEndianしか使わないのでAudioFormat::LinearPcmBe (Be = BigEndian)にはならない.
/// fmtチャンクはwFormatTagによって内容が異なる.
/// https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/Docs/RIFFNEW.pdf
pub(super) fn parse_fmt(input: &[u8]) -> IResult<&[u8], PcmSpecs> {
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
        WaveFormatTag::IeeeFloat => AudioFormat::IeeeFloatLe,
        WaveFormatTag::ImaAdpcm => AudioFormat::ImaAdpcm,
        _ => AudioFormat::Unknown,
    };

    let (input, num_channels) = le_u16(input)?;
    let (input, sample_rate) = le_u32(input)?;
    let (input, _bytes_per_seconds) = le_u32(input)?;
    let (input, block_size) = le_u16(input)?;
    let (input, bit_depth) = le_u16(input)?;

    if audio_format == AudioFormat::ImaAdpcm {
        assert!(block_size % 4 == 0);
        assert!(input.len() >= 4);
        let (input, cb_size) = le_u16(input)?; //2
        assert_eq!(cb_size, 2);
        //wSamplesPerBlock = (((nBlockAlign - (4*nChannels))) * 8) / (wBitPerSample * nChannels) + 1
        let (input, num_samples_per_block) = le_u16(input)?; //2041
        assert_eq!(
            num_samples_per_block,
            ((block_size - (4 * num_channels)) * 8) / (bit_depth * num_channels) + 1
        );

        return Ok((
            input,
            PcmSpecs {
                audio_format,
                num_channels,
                sample_rate,
                bit_depth,
                num_samples_per_block,
            },
        ));
    }

    Ok((
        input,
        PcmSpecs {
            audio_format,
            num_channels,
            sample_rate,
            bit_depth,
            num_samples_per_block: 0,
        },
    ))
}

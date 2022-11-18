use core::convert::TryInto;
use nom::bytes::complete::{tag, take};
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
pub(super) enum ChunkId {
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

impl TryFrom<&[u8]> for ChunkId {
    type Error = ();

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() != 4 {
            return Err(());
        }

        match v {
            b"fmt " => Ok(ChunkId::Fmt),
            b"fact" => Ok(ChunkId::Fact),
            b"PEAK" => Ok(ChunkId::PEAK),
            b"data" => Ok(ChunkId::Data),
            b"junk" => Ok(ChunkId::JUNK),
            b"JUNK" => Ok(ChunkId::JUNK),
            b"IDv3" => Ok(ChunkId::IDv3),
            b"LIST" => Ok(ChunkId::LIST),
            _ => Ok(ChunkId::Unknown),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Chunk<'a> {
    pub id: ChunkId,
    pub size: u32,
    pub data: &'a [u8],
}

/// Waveの形式
/// LinearPCMとIEEE FloatとIMA ADPCMくらいしか使わないはず
/// https://github.com/tpn/winsdk-10/blob/9b69fd26ac0c7d0b83d378dba01080e93349c2ed/Include/10.0.14393.0/shared/mmreg.h#L2107-L2372
#[derive(Debug)]
enum WaveFormatTag {
    Unknown = 0x00,   //0
    LinearPcm = 0x01, //1
    IeeeFloat = 0x03, //3
    ImaAdpcm = 0x11,  //0x11 aka DVI ADPCM
}

impl TryFrom<u16> for WaveFormatTag {
    type Error = ();

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            x if x == WaveFormatTag::LinearPcm as u16 => Ok(WaveFormatTag::LinearPcm),
            x if x == WaveFormatTag::IeeeFloat as u16 => Ok(WaveFormatTag::IeeeFloat),
            x if x == WaveFormatTag::ImaAdpcm as u16 => Ok(WaveFormatTag::ImaAdpcm),
            x if x == WaveFormatTag::Unknown as u16 => Ok(WaveFormatTag::Unknown),
            _ => Err(()),
        }
    }
}

/// RIFFチャンクの情報
///
/// * 'size' - ファイルサイズ(byte) - 8
/// * 'id' - RIFFの識別子 基本"WAVE"
#[derive(Debug)]
pub(super) struct RiffHeader {
    pub size: u32,
}

/// ファイルがRIFFから始まり、識別子がWAVEであることのチェック
pub(super) fn parse_riff_header(input: &[u8]) -> IResult<&[u8], RiffHeader> {
    let (input, _) = tag(b"RIFF")(input)?;
    let (input, size) = le_u32(input)?;
    let (input, _) = tag(b"WAVE")(input)?;
    Ok((input, RiffHeader { size }))
}

pub(super) fn parse_chunk(input: &[u8]) -> IResult<&[u8], Chunk> {
    let (input, chunk_id) = take(4usize)(input)?;
    let id: ChunkId = chunk_id.try_into().unwrap();
    let (input, size) = le_u32(input)?;
    let (input, data) = take(size)(input)?;
    Ok((input, Chunk { id, size, data }))
}

/// WAVはLittleEndianしか使わないのでAudioFormat::LinearPcmBe (Be = BigEndian)にはならない.
/// fmtチャンクはwFormatTagによって内容が異なる.
/// https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/Docs/RIFFNEW.pdf
pub(super) fn parse_fmt(input: &[u8]) -> IResult<&[u8], PcmSpecs> {
    let (input, wave_format_tag) = le_u16(input)?;
    let audio_format = match wave_format_tag.try_into() {
        Ok(WaveFormatTag::Unknown) => AudioFormat::Unknown,
        Ok(WaveFormatTag::LinearPcm) => AudioFormat::LinearPcmLe,
        Ok(WaveFormatTag::IeeeFloat) => AudioFormat::IeeeFloatLe,
        Ok(WaveFormatTag::ImaAdpcm) => AudioFormat::ImaAdpcm,
        Err(_) => AudioFormat::Unknown,
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
        },
    ))
}

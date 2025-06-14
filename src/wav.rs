use crate::{AudioFormat, PcmReaderError, PcmSpecs};
use winnow::binary::{le_u16, le_u32};
use winnow::token::{literal, take};
use winnow::{ModalResult, Parser};

/// WAVのchunkの種類
#[derive(Debug, PartialEq, Default)]
pub(super) enum ChunkId {
    Fmt,  // b"fmt "
    Fact, // b"fact"
    Peak, // b"PEAK"
    Data, // b"data"
    Junk,
    List,
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
            b"PEAK" => Ok(ChunkId::Peak),
            b"data" => Ok(ChunkId::Data),
            b"junk" => Ok(ChunkId::Junk),
            b"JUNK" => Ok(ChunkId::Junk),
            b"IDv3" => Ok(ChunkId::IDv3),
            b"LIST" => Ok(ChunkId::List),
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

/// Waveの形式
/// LinearPCMとIEEE FloatとIMA-ADPCMくらいしか使わないはず
/// https://github.com/tpn/winsdk-10/blob/9b69fd26ac0c7d0b83d378dba01080e93349c2ed/Include/10.0.14393.0/shared/mmreg.h#L2107-L2372
#[derive(Debug, PartialEq)]
enum WaveFormatTag {
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
            _ => Err(()),
        }
    }
}

/// RIFFチャンクの情報
/// * 'size' - ファイルサイズ(byte)-8
#[derive(Debug)]
pub(super) struct RiffHeader {
    pub size: u32,
}

/// ファイルがRIFFから始まり、識別子がWAVEであることのチェック
pub(super) fn parse_riff_header(input: &mut &[u8]) -> ModalResult<RiffHeader> {
    literal(b"RIFF").parse_next(input)?;
    let size = le_u32.parse_next(input)?;
    literal(b"WAVE").parse_next(input)?;
    Ok(RiffHeader { size })
}

pub(super) fn parse_chunk<'a>(input: &mut &'a [u8]) -> ModalResult<Chunk<'a>> {
    let id: ChunkId = take(4usize)
        .map(|id: &'a [u8]| {
            let id: ChunkId = id.try_into().unwrap();
            id
        })
        .parse_next(input)?;
    let size = le_u32.parse_next(input)?;
    let data = take(size).parse_next(input)?;
    Ok(Chunk { id, size, data })
}

/// WAVのfmtチャンクから取得できる情報の構造体
/// * 'audio_format' - LinearPCM or IEEE Float or IMA-ADPCM.
/// * 'num_channels' - Mono: 1, Stereo: 2, and so on.
/// * 'sample_rate' - Sample rate in Hz (44100, 48000, etc...).
/// * 'bit_depth' - Bit depth (16, 24, 32, etc...).
/// * 'ima_adpcm_num_block_align' - IMA-ADPCM only. IMA-ADPCMの1ブロックが何byteで構成されているか。
/// * 'ima_adpcm_num_samples_per_block' - IMA-ADPCM only. IMA-ADPCMの1ブロックに何サンプル記録されているか。
#[derive(Debug, Default)]
pub(super) struct WavFmtSpecs {
    pub audio_format: AudioFormat,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub ima_adpcm_num_block_align: Option<u16>,
    pub ima_adpcm_num_samples_per_block: Option<u16>,
}

/// WAVはLittleEndianしか使わないのでAudioFormat::LinearPcmBe (Be = BigEndian)にはならない.
/// fmtチャンクはwFormatTagによって拡張属性が追加される場合がある.
/// https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/Docs/RIFFNEW.pdf
pub(super) fn parse_fmt(input: &mut &[u8]) -> ModalResult<WavFmtSpecs> {
    let wave_format_tag = le_u16.parse_next(input)?;
    let audio_format = match wave_format_tag.try_into().unwrap() {
        WaveFormatTag::LinearPcm => AudioFormat::LinearPcmLe,
        WaveFormatTag::IeeeFloat => AudioFormat::IeeeFloatLe,
        WaveFormatTag::ImaAdpcm => AudioFormat::ImaAdpcmLe,
    };

    let num_channels = le_u16.parse_next(input)?;
    let sample_rate = le_u32.parse_next(input)?;
    let _bytes_per_seconds = le_u32.parse_next(input)?;
    let block_size = le_u16
        .verify(|block_size| {
            // IMA_ADPCMのときはblock_size(num_block_align)は4の倍数でなければならない
            match audio_format {
                AudioFormat::ImaAdpcmLe => *block_size % 4 == 0,
                _ => true,
            }
        })
        .parse_next(input)?;
    let bit_depth = le_u16.parse_next(input)?;

    if audio_format == AudioFormat::ImaAdpcmLe {
        //IMA-ADPCMの拡張属性の取得
        let num_block_align = block_size;

        let _cb_size = le_u16.verify(|cb_size| *cb_size == 2).parse_next(input)?;

        // wSamplesPerBlock = (((nBlockAlign - (4*nChannels))) * 8) / (wBitPerSample * nChannels) + 1
        let num_samples_per_block = le_u16
            .verify(|num_samples_per_block| {
                *num_samples_per_block
                    == ((num_block_align - (4 * num_channels)) * 8) / (bit_depth * num_channels) + 1
            })
            .parse_next(input)?; //2041

        return Ok(WavFmtSpecs {
            audio_format,
            num_channels,
            sample_rate,
            bit_depth,
            ima_adpcm_num_block_align: Some(block_size),
            ima_adpcm_num_samples_per_block: Some(num_samples_per_block),
        });
    }

    Ok(WavFmtSpecs {
        audio_format,
        num_channels,
        sample_rate,
        bit_depth,
        ima_adpcm_num_block_align: None,
        ima_adpcm_num_samples_per_block: None,
    })
}

/// dataチャンクのサイズ情報からサンプル数を求める
/// IMA-ADPCMは非対応。fmtチャンクの拡張属性から取得する必要がある。
/// * 'data_chunk_size_in_bytes' - dataチャンクのlength (byte)
/// * 'spec' - PCMファイルの情報
pub(super) fn calc_num_samples_per_channel(
    data_chunk_size_in_bytes: u32,
    spec: &PcmSpecs,
) -> Result<u32, PcmReaderError> {
    // IMA-ADPCMは非対応
    if spec.audio_format == AudioFormat::ImaAdpcmLe {
        return Err(PcmReaderError::UnsupportedAudioFormat);
    }

    Ok(data_chunk_size_in_bytes / (spec.bit_depth / 8u16 * spec.num_channels) as u32)
}

#[cfg(test)]
mod tests {
    use crate::{PcmSpecs, wav::ChunkId, wav::calc_num_samples_per_channel};

    use super::WaveFormatTag;

    #[test]
    fn calc_num_samples() {
        let spec = PcmSpecs {
            audio_format: crate::AudioFormat::LinearPcmLe,
            bit_depth: 16,
            num_channels: 2,
            ..Default::default()
        };
        let n = calc_num_samples_per_channel(192000, &spec).unwrap();
        assert_eq!(n, 48000);

        // IMA-ADPCMのときにErrになるかtest
        let spec = PcmSpecs {
            audio_format: crate::AudioFormat::ImaAdpcmLe,
            bit_depth: 4,
            num_channels: 1,
            ..Default::default()
        };
        let e = calc_num_samples_per_channel(2041, &spec);
        assert!(e.is_err());
    }

    #[test]
    fn wave_format_tag_test() {
        let b = 0x01;
        let tag: WaveFormatTag = b.try_into().unwrap();
        assert_eq!(tag, WaveFormatTag::LinearPcm);

        let b = 0x03;
        let tag: WaveFormatTag = b.try_into().unwrap();
        assert_eq!(tag, WaveFormatTag::IeeeFloat);

        let b = 0x11;
        let tag: WaveFormatTag = b.try_into().unwrap();
        assert_eq!(tag, WaveFormatTag::ImaAdpcm);

        let b = 0xFF;
        let e: Result<WaveFormatTag, ()> = b.try_into();
        assert_eq!(e, Err(()));
    }

    #[test]
    fn chunk_id_test() {
        let b = b"fmt ";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Fmt);

        let b = b"fact";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Fact);

        let b = b"PEAK";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Peak);

        let b = b"data";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Data);

        let b = b"JUNK";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Junk);

        let b = b"junk";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Junk);

        let b = b"IDv3";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::IDv3);

        let b = b"LIST";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::List);

        let b = b"HOGE";
        let chunk: ChunkId = b.as_slice().try_into().unwrap();
        assert_eq!(chunk, ChunkId::Unknown);

        let b = b"FOO";
        let e: Result<ChunkId, ()> = b.as_slice().try_into();
        assert_eq!(e, Err(()));
    }
}

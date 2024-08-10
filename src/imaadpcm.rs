//! IMA-ADPCM
//!
//! # Examples
//!
//! Play a IMA-ADPCM file.
//! ```
//! use pacmog::imaadpcm::{ImaAdpcmPlayer, I1F15};
//!
//! let data = include_bytes!("../tests/resources/Sine440Hz_1ch_48000Hz_4bit_IMAADPCM.wav");
//! let mut player = ImaAdpcmPlayer::new(data);
//! let mut buffer: [I1F15; 2] = [I1F15::ZERO, I1F15::ZERO];
//! let b = buffer.as_mut_slice();
//!
//! for _ in 0..48000 {
//!     player.get_next_frame(b).unwrap();
//! }
//! ```

use crate::{error, AudioFormat, PcmReader, PcmSpecs};
use arbitrary_int::u4;
// use fixed::types::I1F15;
use heapless::spsc::Queue;
use nom::bits::{bits, complete::take};
use nom::number::complete::{le_i16, le_i8, le_u8};
use nom::sequence::tuple;
use nom::IResult;

pub use fixed::types::I1F15;

/// Index table for STEP_SIZE_TABLE.
const INDEX_TABLE: [i8; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

/// Quantizer lookup table for decode IMA-ADPCM.
const STEP_SIZE_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

const MAX_NUM_CHANNELS: usize = 2;

/// IMA-ADPCMの各ブロックのHeaderから読み出す情報.
/// * 'i_samp_0' - The first sample value of the block. When decoding, this will be used as the previous sample to start decoding with.
/// * 'b_step_table_index' - The current index into the step table array. [0-88]
#[derive(Default, Debug)]
struct BlockHeader {
    i_samp_0: I1F15,
    b_step_table_index: i8,
}

/// IMA-ADPCMのHeader Wordをパースする
/// Multimedia Data Standards Update April 15, 1994 Page 32 of 74
/// http://elm-chan.org/junk/adpcm/RIFF_NEW.pdf
fn parse_block_header(input: &[u8]) -> IResult<&[u8], BlockHeader> {
    let (input, i_samp_0) = le_i16(input)?;
    let i_samp_0 = I1F15::from_bits(i_samp_0);
    let (input, b_step_table_index) = le_i8(input)?;
    let (input, _reserved) = le_u8(input)?;

    Ok((
        input,
        BlockHeader {
            i_samp_0,
            b_step_table_index,
        },
    ))
}

/// * 'nibble' - 4bit unsigned int data
/// * 'last_predicted_sample' - output of ADPCM predictor [16bitInt]
/// * 'step_size_table_index' - index into step_size_table [0~88]
fn decode_sample(
    nibble: u4,
    last_predicted_sample: I1F15,
    step_size_table_index: i8,
) -> (I1F15, i8) {
    // calculate difference = (originalSample + 1⁄2) * stepsize/4:
    let mut diff = 0i32;
    let step_size = STEP_SIZE_TABLE[step_size_table_index as usize] as i32;
    let n = nibble.value();

    // perform multiplication through repetitive addition
    if (n & 4) == 4 {
        diff += step_size;
    }
    if (n & 2) == 2 {
        diff += step_size >> 1;
    }
    if (n & 1) == 1 {
        diff += step_size >> 2;
    }

    // (originalSample + 1⁄2) * stepsize/4 =originalSample * stepsize/4 + stepsize/8:
    diff += step_size >> 3;

    // account for sign bit
    if (n & 8) == 8 {
        diff = -diff;
    }

    let predicted_sample = last_predicted_sample.saturating_add(I1F15::from_bits(diff as i16));
    let step_size_table_index = compute_step_size(nibble, step_size_table_index);
    (predicted_sample, step_size_table_index)
}

/// Update step_size of IMA-ADPCM table
fn compute_step_size(nibble: u4, mut step_size_table_index: i8) -> i8 {
    // adjust index into step_size lookup table using original_sample
    step_size_table_index += INDEX_TABLE[nibble.value() as usize];
    step_size_table_index = step_size_table_index.clamp(0, 88); //check overflow and underflow
    step_size_table_index
}

/// Calculate the number of samples per channel for IMA-ADPCM files.
pub(crate) fn calc_num_samples_per_channel(
    data_chunk_size_in_bytes: u32,
    spec: &PcmSpecs,
) -> Result<u32, error::DecodingError> {
    if spec.audio_format != AudioFormat::ImaAdpcmLe {
        return Err(error::DecodingError::UnsupportedFormat);
    }

    let num_block_align = spec.ima_adpcm_num_block_align.unwrap() as u32;
    let num_samples_per_block = spec.ima_adpcm_num_samples_per_block.unwrap() as u32;
    let num_blocks = data_chunk_size_in_bytes / num_block_align;
    let num_samples = num_blocks * num_samples_per_block;
    Ok(num_samples)
}

/// High level of organized players for IMA-ADPCM playback.
#[derive(Default)]
pub struct ImaAdpcmPlayer<'a> {
    /// A reader to access basic information about the PCM file.
    pub reader: PcmReader<'a>,
    /// Frame index of the current block.
    frame_index: u32,
    /// The last decoded sample value.
    last_predicted_sample: [I1F15; MAX_NUM_CHANNELS],
    /// The current index of STEP_SIZE_TABLE.
    step_size_table_index: [i8; MAX_NUM_CHANNELS],
    /// The current block of IMA-ADPCM being read.
    reading_block: &'a [u8],
    /// A queue that stores nibble arrays when reading data words.
    /// TODO: Queue size is better to be a power of 2 for performance.
    nibble_queue: [Queue<u4, 9>; MAX_NUM_CHANNELS],
}

impl<'a> ImaAdpcmPlayer<'a> {
    /// * 'input' - PCM data byte array.
    pub fn new(input: &'a [u8]) -> Self {
        let reader = PcmReader::new(input);

        ImaAdpcmPlayer {
            reader,
            frame_index: 0,
            ..Default::default()
        }
    }

    /// Return samples value of the next frame.
    /// * 'out' - Output buffer which the sample values are written. Number of elements must be equal to or greater than the number of channels in the PCM file.
    pub fn get_next_frame(&mut self, out: &mut [I1F15]) -> Result<(), error::PlayerError> {
        let num_channels = self.reader.specs.num_channels;

        // outバッファーのチャンネル数が不足
        if !(out.len() >= num_channels as usize) {
            return Err(error::PlayerError::InvalidData);
        }

        // 再生終了
        if !(self.frame_index < self.reader.specs.num_samples) {
            return Err(error::PlayerError::FinishedPlaying);
        }

        //IMA-ADPCMのBlock切り替わりかどうか判定
        if self.reading_block.is_empty() && self.nibble_queue[0].is_empty() {
            self.update_block();
            out[..(num_channels as usize)]
                .copy_from_slice(&self.last_predicted_sample[..(num_channels as usize)]);
            self.frame_index += 1; //Blockの最初のサンプルはHeaderに記録されている
            return Ok(());
        }

        // 次のData wordsをチャンネル数分よみこむ.
        if self.nibble_queue[0].is_empty() {
            for ch in 0..num_channels as usize {
                let (remains, nibbles) = parse_data_word(self.reading_block).unwrap();
                self.reading_block = remains;
                self.nibble_queue[ch].enqueue(u4::new(nibbles.1)).unwrap();
                self.nibble_queue[ch].enqueue(u4::new(nibbles.0)).unwrap();
                self.nibble_queue[ch].enqueue(u4::new(nibbles.3)).unwrap();
                self.nibble_queue[ch].enqueue(u4::new(nibbles.2)).unwrap();
                self.nibble_queue[ch].enqueue(u4::new(nibbles.5)).unwrap();
                self.nibble_queue[ch].enqueue(u4::new(nibbles.4)).unwrap();
                self.nibble_queue[ch].enqueue(u4::new(nibbles.7)).unwrap();
                self.nibble_queue[ch].enqueue(u4::new(nibbles.6)).unwrap();
            }
        }

        //デコード
        for (ch, output_value) in out.iter_mut().enumerate().take(num_channels as usize) {
            let nibble = self.nibble_queue[ch].dequeue().unwrap();
            let (predicted_sample, table_index) = decode_sample(
                nibble,
                self.last_predicted_sample[ch],
                self.step_size_table_index[ch],
            );
            self.last_predicted_sample[ch] = predicted_sample;
            self.step_size_table_index[ch] = table_index;
            *output_value = predicted_sample;
        }

        self.frame_index += 1;
        Ok(())
    }

    /// IMA-ADPCMのブロック更新.    
    fn update_block(&mut self) {
        let samples_per_block = self.reader.specs.ima_adpcm_num_samples_per_block.unwrap() as u32;
        let block_align = self.reader.specs.ima_adpcm_num_block_align.unwrap() as u32;
        let offset = (self.frame_index / samples_per_block) * block_align;
        self.reading_block = &self.reader.data[offset as usize..(offset + block_align) as usize]; //新しいBlockをreading_blockへ更新

        assert_eq!(self.reading_block.len(), block_align as usize);

        for ch in 0..self.reader.specs.num_channels as usize {
            // BlockのHeader wordを読み出す
            let (block, block_header) = parse_block_header(self.reading_block).unwrap(); //Headerの1ch分は4byte
            self.last_predicted_sample[ch] = block_header.i_samp_0;
            self.step_size_table_index[ch] = block_header.b_step_table_index;
            self.reading_block = block;
        }
    }

    /// Move the playback position back to the beginning.
    pub fn rewind(&mut self) {
        self.frame_index = 0;
        if !self.reading_block.is_empty() {
            self.reading_block = &self.reading_block[0..0]; //reading_blockを空のスライスにする
        }
        for q in &mut self.nibble_queue {
            for _ in 0..q.len() {
                q.dequeue().unwrap();
            }
        }
    }
}

/// IMA-ADPCMのData word (32bit長)を8つのnibble(4bit長)にパースしたもの
type DataWordNibbles = (u8, u8, u8, u8, u8, u8, u8, u8);

/// IMA-ADPCMのBlockのData word（32bit長）を8つのnibble(4bit長)にパースする.
fn parse_data_word(input: &[u8]) -> IResult<&[u8], DataWordNibbles> {
    bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(tuple((
        take(4usize),
        take(4usize),
        take(4usize),
        take(4usize),
        take(4usize),
        take(4usize),
        take(4usize),
        take(4usize),
    )))(input)
}

#[cfg(test)]
mod tests {
    use crate::imaadpcm::{decode_sample, I1F15};
    use arbitrary_int::u4;

    // http://www.cs.columbia.edu/~hgs/audio/dvi/IMA_ADPCM.pdf
    // P.32 4-bit ADPCM to 16-bit Linear Decompression
    #[test]
    fn ima_adpcm_decode() {
        let nibble = u4::new(3);
        let (sample, step_size_table_index) = decode_sample(
            nibble,
            I1F15::from_bits(-30976), //0x8700
            24,
        );
        assert_eq!(sample, I1F15::from_bits(-30913)); //0x873F
        assert_eq!(step_size_table_index, 23);
    }
}

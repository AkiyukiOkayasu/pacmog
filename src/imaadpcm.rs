use nom::{
    number::complete::{le_i16, le_u8},
    IResult,
};

const INDEX_TABLE: [i8; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

// quantizer lookup table
const STEP_SIZE_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

///
/// * 'i_samp_0' - The first sample value of the block. When decoding, this will be used as the previous sample to start decoding with.
/// * 'b_step_table_index' - The current index into the step table array. [0-88]
#[derive(Default, Debug)]
pub struct BlockHeader {
    i_samp_0: i16,
    b_step_table_index: u8,
}

pub(super) fn parse_block_header(input: &[u8]) -> IResult<&[u8], BlockHeader> {
    let (input, i_samp_0) = le_i16(input)?;
    let (input, b_step_table_index) = le_u8(input)?;
    let (input, _reserved) = le_u8(input)?;

    Ok((
        input,
        BlockHeader {
            i_samp_0,
            b_step_table_index,
        },
    ))
}

///

#[derive(Default)]
pub(super) struct ImaAdpcmDecoder {
    predicted_sample: i32,
    step_size_table_index: i8,
    step_size: i32,
}

/// * 'predicted_sample' - output of ADPCM predictor [16bitInt]
/// * 'step_size_table_index' - index into step_size_table [0~88]
/// * 'step_size' - quantizer step size
pub(super) fn decode(
    nibble: u8,
    last_predicted_sample: i16,
    step_size_table_index: i8,
) -> (i16, i8) {
    // calculate difference = (originalSample + 1⁄2) * stepsize/4:
    let mut diff = 0i32;
    let step_size = STEP_SIZE_TABLE[step_size_table_index as usize] as i32;

    // perform multiplication through repetitive addition
    if (last_predicted_sample & 4) == 4 {
        diff += step_size;
    }
    if (last_predicted_sample & 2) == 2 {
        diff += step_size >> 1;
    }
    if (last_predicted_sample & 1) == 1 {
        diff += step_size >> 2;
    }

    // (originalSample + 1⁄2) * stepsize/4 =originalSample * stepsize/4 + stepsize/8:
    diff += step_size >> 3;

    // account for sign bit
    if (last_predicted_sample & 8) == 8 {
        diff -= diff;
    }

    let mut predicted_sample = last_predicted_sample as i32 + diff; // adjust predicted sample based on calculated difference:
    predicted_sample = predicted_sample.clamp(-32768, 32767); // check for overflow and underflow

    let step_size_table_index = compute_step_size(nibble, step_size_table_index);

    (predicted_sample as i16, step_size_table_index)
}

/// step_sizeの更新
fn compute_step_size(nibble: u8, mut step_size_table_index: i8) -> i8 {
    // adjust index into step_size lookup table using original_sample
    step_size_table_index += INDEX_TABLE[nibble as usize];
    step_size_table_index = step_size_table_index.clamp(0, 88); //check overflow and underflow
    step_size_table_index
}

impl ImaAdpcmDecoder {
    /// 4bit IMA-ADPCM to 16bit Int
    /// compute predicted sample estimate newSample
    fn decode(&mut self, original_sample: u8) -> i32 {
        // calculate difference = (originalSample + 1⁄2) * stepsize/4:
        let mut diff = 0i32;

        // perform multiplication through repetitive addition
        if (original_sample & 4) == 4 {
            diff += self.step_size;
        }
        if (original_sample & 2) == 2 {
            diff += self.step_size >> 1;
            println!("true: {}", diff);
        }
        if (original_sample & 1) == 1 {
            diff += self.step_size >> 2;
        }

        // (originalSample + 1⁄2) * stepsize/4 =originalSample * stepsize/4 + stepsize/8:
        diff += self.step_size >> 3;

        // account for sign bit
        if (original_sample & 8) == 8 {
            diff -= diff;
        }

        self.predicted_sample += diff; // adjust predicted sample based on calculated difference:
        self.predicted_sample = self.predicted_sample.clamp(-32768, 32767); // check for overflow and underflow

        self.compute_step_size(original_sample);
        self.predicted_sample
    }

    /// step_sizeの更新
    fn compute_step_size(&mut self, nibble: u8) {
        // adjust index into step_size lookup table using original_sample
        self.step_size_table_index += INDEX_TABLE[nibble as usize];
        self.step_size_table_index = self.step_size_table_index.clamp(0, 88); //check overflow and underflow

        self.step_size = STEP_SIZE_TABLE[self.step_size_table_index as usize] as i32;
    }
}

#[cfg(test)]
mod tests {
    use super::ImaAdpcmDecoder;

    #[test]
    fn test() {
        let mut dec = ImaAdpcmDecoder::default();
        dec.predicted_sample = -30976;
        dec.step_size = 73;
        dec.step_size_table_index = 24;
        let sample = dec.decode(0x3);

        assert_eq!(sample, -30913); //0x873F
        assert_eq!(dec.step_size, 66);
        assert_eq!(dec.step_size_table_index, 23);
    }
}

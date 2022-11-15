const index_table: [i8; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

// quantizer lookup table
const step_size_table: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

///
/// * 'predicted_sample' - output of ADPCM predictor [16bitInt]
/// * 'step_size_table_index' - index into step_size_table [0~88]
/// * 'step_size' - quantizer step size
#[derive(Default)]
pub struct ImaAdpcmDecoder {
    predicted_sample: i32,
    step_size_table_index: i8,
    step_size: i32,
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
        self.step_size_table_index += index_table[nibble as usize];
        self.step_size_table_index = self.step_size_table_index.clamp(0, 88); //check overflow and underflow

        self.step_size = step_size_table[self.step_size_table_index as usize] as i32;
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

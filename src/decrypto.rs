use std::io::Read;
use flate2::read::ZlibDecoder;
use crate::AppError;

pub const ENCRYPT: u32 = 1;
pub const DECRYPT: u32 = 0;

pub const QQ_KEY: &[u8] = b"!@#)(*$%123ZXC!@!@#)(NHL";
pub const SBOX1: [u8; 64] = [
    14,  4,  13,  1,   2, 15,  11,  8,   3, 10,   6, 12,   5,  9,   0,  7,
     0, 15,   7,  4,  14,  2,  13,  1,  10,  6,  12, 11,   9,  5,   3,  8,
     4,  1,  14,  8,  13,  6,   2, 11,  15, 12,   9,  7,   3, 10,   5,  0,
    15, 12,   8,  2,   4,  9,   1,  7,   5, 11,   3, 14,  10,  0,   6, 13
];

pub const SBOX2: [u8; 64] = [
    15,  1,   8, 14,   6, 11,   3,  4,   9,  7,   2, 13,  12,  0,   5, 10,
     3, 13,   4,  7,  15,  2,   8, 15,  12,  0,   1, 10,   6,  9,  11,  5,
     0, 14,   7, 11,  10,  4,  13,  1,   5,  8,  12,  6,   9,  3,   2, 15,
    13,  8,  10,  1,   3, 15,   4,  2,  11,  6,   7, 12,   0,  5,  14,  9
];

pub const SBOX3: [u8; 64] = [
    10,  0,   9, 14,   6,  3,  15,  5,   1, 13,  12,  7,  11,  4,   2,  8,
    13,  7,   0,  9,   3,  4,   6, 10,   2,  8,   5, 14,  12, 11,  15,  1,
    13,  6,   4,  9,   8, 15,   3,  0,  11,  1,   2, 12,   5, 10,  14,  7,
     1, 10,  13,  0,   6,  9,   8,  7,   4, 15,  14,  3,  11,  5,   2, 12
];

pub const SBOX4: [u8; 64] = [
     7, 13,  14,  3,   0,  6,   9, 10,   1,  2,   8,  5,  11, 12,   4, 15,
    13,  8,  11,  5,   6, 15,   0,  3,   4,  7,   2, 12,   1, 10,  14,  9,
    10,  6,   9,  0,  12, 11,   7, 13,  15,  1,   3, 14,   5,  2,   8,  4,
     3, 15,   0,  6,  10, 10,  13,  8,   9,  4,   5, 11,  12,  7,   2, 14
];

pub const SBOX5: [u8; 64] = [
     2, 12,   4,  1,   7, 10,  11,  6,   8,  5,   3, 15,  13,  0,  14,  9,
    14, 11,   2, 12,   4,  7,  13,  1,   5,  0,  15, 10,   3,  9,   8,  6,
     4,  2,   1, 11,  10, 13,   7,  8,  15,  9,  12,  5,   6,  3,   0, 14,
    11,  8,  12,  7,   1, 14,   2, 13,   6, 15,   0,  9,  10,  4,   5,  3
];

pub const SBOX6: [u8; 64] = [
    12,  1,  10, 15,   9,  2,   6,  8,   0, 13,   3,  4,  14,  7,   5, 11,
    10, 15,   4,  2,   7, 12,   9,  5,   6,  1,  13, 14,   0, 11,   3,  8,
     9, 14,  15,  5,   2,  8,  12,  3,   7,  0,   4, 10,   1, 13,  11,  6,
     4,  3,   2, 12,   9,  5,  15, 10,  11, 14,   1,  7,   6,  0,   8, 13
];

pub const SBOX7: [u8; 64] = [
     4, 11,   2, 14,  15,  0,   8, 13,   3, 12,   9,  7,   5, 10,   6,  1,
    13,  0,  11,  7,   4,  9,   1, 10,  14,  3,   5, 12,   2, 15,   8,  6,
     1,  4,  11, 13,  12,  3,   7, 14,  10, 15,   6,  8,   0,  5,   9,  2,
     6, 11,  13,  8,   1,  4,  10,  7,   9,  5,   0, 15,  14,  2,   3, 12
];

pub const SBOX8: [u8; 64] = [
    13,  2,   8,  4,   6, 15,  11,  1,  10,  9,   3, 14,   5,  0,  12,  7,
     1, 15,  13,  8,  10,  3,   7,  4,  12,  5,   6, 11,   0, 14,   9,  2,
     7, 11,   4,  1,   9, 12,  14,  2,   0,  6,  10, 13,  15,  3,   5,  8,
     2,  1,  14,  7,   4, 10,   8, 13,  15, 12,   9,  0,   3,  5,   6, 11
];

pub const fn bit_num(a: &[u8], b: usize, c: usize) -> u32 {
    ((a[b / 32 * 4 + 3 - b % 32 / 8] >> (7 - (b % 8))) & 0x01) as u32 * (1 << c)
}

pub const fn bit_num_intr(a: u32, b: usize, c: usize) -> u8 {
    (((a >> (31 - b)) & 0x00000001) << c) as u8
}

pub const fn bit_num_intl(a: u32, b: usize, c: usize) -> u32 {
    ((a << b) & 0x80000000) >> c
}

pub const fn sbox_bit(a: u8) -> usize {
    ((a & 0x20) | ((a & 0x1f) >> 1) | ((a & 0x01) << 4)) as usize
}

pub fn key_schedule(key: &[u8], schedule: &mut [Vec<u8>], mode: u32) {
    let key_rnd_shift: [u32; 16] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];
    let key_perm_c: [usize; 28] = [56, 48, 40, 32, 24, 16, 8, 0, 57, 49, 41, 33, 25, 17,
        9, 1, 58, 50, 42, 34, 26, 18, 10, 2, 59, 51, 43, 35];
    let key_perm_d: [usize; 28] = [62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29, 21,
        13, 5, 60, 52, 44, 36, 28, 20, 12, 4, 27, 19, 11, 3];
    let key_compression: [usize; 48] = [13, 16, 10, 23, 0, 4, 2, 27, 14, 5, 20, 9,
        22, 18, 11, 3, 25, 7, 15, 6, 26, 19, 12, 1,
        40, 51, 30, 36, 46, 54, 29, 39, 50, 44, 32, 47,
        43, 48, 38, 55, 33, 52, 45, 41, 49, 35, 28, 31];

    let mut c = 0u32;
    let mut d = 0u32;

    for (i, &perm) in key_perm_c.iter().enumerate() {
        c |= bit_num(key, perm, 31 - i);
    }

    for (i, &perm) in key_perm_d.iter().enumerate() {
        d |= bit_num(key, perm, 31 - i);
    }

    for (i, &shift) in key_rnd_shift.iter().enumerate() {
        c = ((c << shift as usize) | (c >> (28 - shift as usize))) & 0xfffffff0;
        d = ((d << shift as usize) | (d >> (28 - shift as usize))) & 0xfffffff0;

        let to_gen = if mode == DECRYPT { 15 - i } else { i };

        for j in 0..6 {
            schedule[to_gen][j] = 0;
        }

        for (j, &comp) in key_compression.iter().enumerate().take(24) {
            schedule[to_gen][j / 8] |= bit_num_intr(c, comp, 7 - (j % 8));
        }

        for (j, &comp) in key_compression.iter().enumerate().skip(24) {
            schedule[to_gen][j / 8] |= bit_num_intr(d, comp - 27, 7 - (j % 8));
        }
    }
}

pub fn ip(state: &mut [u32; 2], input: &[u8]) {
    state[0] = bit_num(input, 57, 31) | bit_num(input, 49, 30) | bit_num(input, 41, 29) | bit_num(input, 33, 28) |
        bit_num(input, 25, 27) | bit_num(input, 17, 26) | bit_num(input, 9, 25) | bit_num(input, 1, 24) |
        bit_num(input, 59, 23) | bit_num(input, 51, 22) | bit_num(input, 43, 21) | bit_num(input, 35, 20) |
        bit_num(input, 27, 19) | bit_num(input, 19, 18) | bit_num(input, 11, 17) | bit_num(input, 3, 16) |
        bit_num(input, 61, 15) | bit_num(input, 53, 14) | bit_num(input, 45, 13) | bit_num(input, 37, 12) |
        bit_num(input, 29, 11) | bit_num(input, 21, 10) | bit_num(input, 13, 9) | bit_num(input, 5, 8) |
        bit_num(input, 63, 7) | bit_num(input, 55, 6) | bit_num(input, 47, 5) | bit_num(input, 39, 4) |
        bit_num(input, 31, 3) | bit_num(input, 23, 2) | bit_num(input, 15, 1) | bit_num(input, 7, 0);

    state[1] = bit_num(input, 56, 31) | bit_num(input, 48, 30) | bit_num(input, 40, 29) | bit_num(input, 32, 28) |
        bit_num(input, 24, 27) | bit_num(input, 16, 26) | bit_num(input, 8, 25) | bit_num(input, 0, 24) |
        bit_num(input, 58, 23) | bit_num(input, 50, 22) | bit_num(input, 42, 21) | bit_num(input, 34, 20) |
        bit_num(input, 26, 19) | bit_num(input, 18, 18) | bit_num(input, 10, 17) | bit_num(input, 2, 16) |
        bit_num(input, 60, 15) | bit_num(input, 52, 14) | bit_num(input, 44, 13) | bit_num(input, 36, 12) |
        bit_num(input, 28, 11) | bit_num(input, 20, 10) | bit_num(input, 12, 9) | bit_num(input, 4, 8) |
        bit_num(input, 62, 7) | bit_num(input, 54, 6) | bit_num(input, 46, 5) | bit_num(input, 38, 4) |
        bit_num(input, 30, 3) | bit_num(input, 22, 2) | bit_num(input, 14, 1) | bit_num(input, 6, 0);
}

pub fn inv_ip(state: &[u32; 2], output: &mut [u8]) {
    output[3] = bit_num_intr(state[1], 7, 7) | bit_num_intr(state[0], 7, 6) | bit_num_intr(state[1], 15, 5) |
        bit_num_intr(state[0], 15, 4) | bit_num_intr(state[1], 23, 3) | bit_num_intr(state[0], 23, 2) |
        bit_num_intr(state[1], 31, 1) | bit_num_intr(state[0], 31, 0);

    output[2] = bit_num_intr(state[1], 6, 7) | bit_num_intr(state[0], 6, 6) | bit_num_intr(state[1], 14, 5) |
        bit_num_intr(state[0], 14, 4) | bit_num_intr(state[1], 22, 3) | bit_num_intr(state[0], 22, 2) |
        bit_num_intr(state[1], 30, 1) | bit_num_intr(state[0], 30, 0);

    output[1] = bit_num_intr(state[1], 5, 7) | bit_num_intr(state[0], 5, 6) | bit_num_intr(state[1], 13, 5) |
        bit_num_intr(state[0], 13, 4) | bit_num_intr(state[1], 21, 3) | bit_num_intr(state[0], 21, 2) |
        bit_num_intr(state[1], 29, 1) | bit_num_intr(state[0], 29, 0);

    output[0] = bit_num_intr(state[1], 4, 7) | bit_num_intr(state[0], 4, 6) | bit_num_intr(state[1], 12, 5) |
        bit_num_intr(state[0], 12, 4) | bit_num_intr(state[1], 20, 3) | bit_num_intr(state[0], 20, 2) |
        bit_num_intr(state[1], 28, 1) | bit_num_intr(state[0], 28, 0);

    output[7] = bit_num_intr(state[1], 3, 7) | bit_num_intr(state[0], 3, 6) | bit_num_intr(state[1], 11, 5) |
        bit_num_intr(state[0], 11, 4) | bit_num_intr(state[1], 19, 3) | bit_num_intr(state[0], 19, 2) |
        bit_num_intr(state[1], 27, 1) | bit_num_intr(state[0], 27, 0);

    output[6] = bit_num_intr(state[1], 2, 7) | bit_num_intr(state[0], 2, 6) | bit_num_intr(state[1], 10, 5) |
        bit_num_intr(state[0], 10, 4) | bit_num_intr(state[1], 18, 3) | bit_num_intr(state[0], 18, 2) |
        bit_num_intr(state[1], 26, 1) | bit_num_intr(state[0], 26, 0);

    output[5] = bit_num_intr(state[1], 1, 7) | bit_num_intr(state[0], 1, 6) | bit_num_intr(state[1], 9, 5) |
        bit_num_intr(state[0], 9, 4) | bit_num_intr(state[1], 17, 3) | bit_num_intr(state[0], 17, 2) |
        bit_num_intr(state[1], 25, 1) | bit_num_intr(state[0], 25, 0);

    output[4] = bit_num_intr(state[1], 0, 7) | bit_num_intr(state[0], 0, 6) | bit_num_intr(state[1], 8, 5) |
        bit_num_intr(state[0], 8, 4) | bit_num_intr(state[1], 16, 3) | bit_num_intr(state[0], 16, 2) |
        bit_num_intr(state[1], 24, 1) | bit_num_intr(state[0], 24, 0);
}

pub fn f_function(state: u32, key: &[u8]) -> u32 {
    let mut lrg_state = [0u8; 6];
    let t1 = bit_num_intl(state, 31, 0) | ((state & 0xf0000000) >> 1) | bit_num_intl(state, 4, 5) |
        bit_num_intl(state, 3, 6) | ((state & 0x0f000000) >> 3) | bit_num_intl(state, 8, 11) |
        bit_num_intl(state, 7, 12) | ((state & 0x00f00000) >> 5) | bit_num_intl(state, 12, 17) |
        bit_num_intl(state, 11, 18) | ((state & 0x000f0000) >> 7) | bit_num_intl(state, 16, 23);

    let t2 = bit_num_intl(state, 15, 0) | ((state & 0x0000f000) << 15) | bit_num_intl(state, 20, 5) |
        bit_num_intl(state, 19, 6) | ((state & 0x00000f00) << 13) | bit_num_intl(state, 24, 11) |
        bit_num_intl(state, 23, 12) | ((state & 0x000000f0) << 11) | bit_num_intl(state, 28, 17) |
        bit_num_intl(state, 27, 18) | ((state & 0x0000000f) << 9) | bit_num_intl(state, 0, 23);

    lrg_state[0] = ((t1 >> 24) & 0x000000ff) as u8;
    lrg_state[1] = ((t1 >> 16) & 0x000000ff) as u8;
    lrg_state[2] = ((t1 >> 8) & 0x000000ff) as u8;
    lrg_state[3] = ((t2 >> 24) & 0x000000ff) as u8;
    lrg_state[4] = ((t2 >> 16) & 0x000000ff) as u8;
    lrg_state[5] = ((t2 >> 8) & 0x000000ff) as u8;

    lrg_state[0] ^= key[0];
    lrg_state[1] ^= key[1];
    lrg_state[2] ^= key[2];
    lrg_state[3] ^= key[3];
    lrg_state[4] ^= key[4];
    lrg_state[5] ^= key[5];

    let mut result = ((SBOX1[sbox_bit(lrg_state[0] >> 2)] as u32) << 28) |
        ((SBOX2[sbox_bit(((lrg_state[0] & 0x03) << 4) | (lrg_state[1] >> 4))] as u32) << 24) |
        ((SBOX3[sbox_bit(((lrg_state[1] & 0x0f) << 2) | (lrg_state[2] >> 6))] as u32) << 20) |
        ((SBOX4[sbox_bit(lrg_state[2] & 0x3f)] as u32) << 16) |
        ((SBOX5[sbox_bit(lrg_state[3] >> 2)] as u32) << 12) |
        ((SBOX6[sbox_bit(((lrg_state[3] & 0x03) << 4) | (lrg_state[4] >> 4))] as u32) << 8) |
        ((SBOX7[sbox_bit(((lrg_state[4] & 0x0f) << 2) | (lrg_state[5] >> 6))] as u32) << 4) |
        (SBOX8[sbox_bit(lrg_state[5] & 0x3f)] as u32);

    result = bit_num_intl(result, 15, 0) | bit_num_intl(result, 6, 1) | bit_num_intl(result, 19, 2) |
        bit_num_intl(result, 20, 3) | bit_num_intl(result, 28, 4) | bit_num_intl(result, 11, 5) |
        bit_num_intl(result, 27, 6) | bit_num_intl(result, 16, 7) | bit_num_intl(result, 0, 8) |
        bit_num_intl(result, 14, 9) | bit_num_intl(result, 22, 10) | bit_num_intl(result, 25, 11) |
        bit_num_intl(result, 4, 12) | bit_num_intl(result, 17, 13) | bit_num_intl(result, 30, 14) |
        bit_num_intl(result, 9, 15) | bit_num_intl(result, 1, 16) | bit_num_intl(result, 7, 17) |
        bit_num_intl(result, 23, 18) | bit_num_intl(result, 13, 19) | bit_num_intl(result, 31, 20) |
        bit_num_intl(result, 26, 21) | bit_num_intl(result, 2, 22) | bit_num_intl(result, 8, 23) |
        bit_num_intl(result, 18, 24) | bit_num_intl(result, 12, 25) | bit_num_intl(result, 29, 26) |
        bit_num_intl(result, 5, 27) | bit_num_intl(result, 21, 28) | bit_num_intl(result, 10, 29) |
        bit_num_intl(result, 3, 30) | bit_num_intl(result, 24, 31);

    result
}

pub fn des_crypt(input: &[u8], output: &mut [u8], key: &[Vec<u8>]) {
    let mut state = [0u32; 2];
    
    ip(&mut state, input);

    for key_item in key.iter().take(15) {
        let t = state[1];
        state[1] = f_function(state[1], key_item) ^ state[0];
        state[0] = t;
    }

    state[0] ^= f_function(state[1], &key[15]);

    inv_ip(&state, output);
}

pub fn triple_des_key_setup(key: &[u8], schedule: &mut [Vec<Vec<u8>>], mode: u32) {
    if mode == ENCRYPT {
        key_schedule(&key[0..8], &mut schedule[0], mode);
        key_schedule(&key[8..16], &mut schedule[1], DECRYPT);
        key_schedule(&key[16..24], &mut schedule[2], mode);
    } else {
        key_schedule(&key[0..8], &mut schedule[2], mode);
        key_schedule(&key[8..16], &mut schedule[1], ENCRYPT);
        key_schedule(&key[16..24], &mut schedule[0], mode);
    }
}

pub fn triple_des_crypt(input: &[u8], output: &mut [u8], key: &[Vec<Vec<u8>>]) {
    let mut temp1 = [0u8; 8];
    let mut temp2 = [0u8; 8];
    
    des_crypt(input, &mut temp1, &key[0]);
    des_crypt(&temp1, &mut temp2, &key[1]);
    des_crypt(&temp2, output, &key[2]);
}

pub fn hex_string_to_byte_array(hex_string: &str) -> Result<Vec<u8>, AppError> {
    (0..hex_string.len())
        .step_by(2)
        .filter_map(|i| {
            if i + 2 <= hex_string.len() {
                let byte_str = &hex_string[i..i+2];
                match u8::from_str_radix(byte_str, 16) {
                    Ok(byte) => Some(Ok(byte)),
                    Err(e) => Some(Err(AppError::InvalidHex(e))),
                }
            } else {
                None
            }
        })
        .collect()
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(AppError::Decompression)?;
    if decompressed.len() >= 3 && decompressed[0] == 0xEF && decompressed[1] == 0xBB && decompressed[2] == 0xBF {
        Ok(decompressed[3..].to_vec())
    } else {
        Ok(decompressed)
    }
}

pub fn decrypt_lyrics(encrypted: &str) -> Result<String, AppError> {
    let encrypted_bytes = hex_string_to_byte_array(encrypted)?;
    let mut decrypted_data = vec![0; encrypted_bytes.len()];
    
    let mut schedule = vec![vec![vec![0u8; 6]; 16]; 3];
    triple_des_key_setup(QQ_KEY, &mut schedule, DECRYPT); 
    
    for (i, chunk) in encrypted_bytes.chunks(8).enumerate() {
        if chunk.len() == 8 {
            let mut temp = [0u8; 8];
            triple_des_crypt(chunk, &mut temp, &schedule);
            
            let start = i * 8;
            let end = start + 8;
            if end <= decrypted_data.len() {
                decrypted_data[start..end].copy_from_slice(&temp);
            }
        }
    }
    
    let decompressed = decompress(&decrypted_data)?;
    String::from_utf8(decompressed).map_err(AppError::Utf8)
}
//! A QR code for a short text: version 6 (41 x 41 modules), error correction level Q, byte mode, mask 0. Level Q lets
//! a quarter of the code be hidden, which leaves room for a logo in the middle. Enough for a NEB address (62
//! characters); the limit is 74 bytes.

pub const SIZE: usize = 41;
const BLOCKS: usize = 4;
const DATA_PER_BLOCK: usize = 19;
const EC_PER_BLOCK: usize = 24;

/// The modules of a code, `[row][column]`, true for a dark one.
pub type Modules = [[bool; SIZE]; SIZE];

/// The code of `text`, or `None` when the text is longer than 74 bytes.
pub fn encode(text: &str) -> Option<Modules> {
    let bytes = text.as_bytes();
    let capacity = BLOCKS * DATA_PER_BLOCK;
    if bytes.len() > capacity - 2 {
        return None;
    }
    // Byte mode: 4 bits of mode, 8 bits of length, the bytes, a terminator, then padding bytes.
    let mut data = vec![0u8; capacity];
    let mut bit = 0;
    put(&mut data, &mut bit, 0x4, 4);
    put(&mut data, &mut bit, bytes.len() as u32, 8);
    for b in bytes {
        put(&mut data, &mut bit, u32::from(*b), 8);
    }
    put(&mut data, &mut bit, 0, 4);
    let used = bit.div_ceil(8);
    for (i, slot) in data.iter_mut().enumerate().skip(used) {
        *slot = if (i - used) % 2 == 0 { 0xEC } else { 0x11 };
    }

    // Four blocks, each followed by its error correction bytes; the bytes of the blocks are then interleaved.
    let generator = generator(EC_PER_BLOCK);
    let ec: Vec<Vec<u8>> = data.chunks(DATA_PER_BLOCK).map(|block| remainder(block, &generator)).collect();
    let mut all = Vec::with_capacity(BLOCKS * (DATA_PER_BLOCK + EC_PER_BLOCK));
    for i in 0..DATA_PER_BLOCK {
        all.extend((0..BLOCKS).map(|b| data[b * DATA_PER_BLOCK + i]));
    }
    for i in 0..EC_PER_BLOCK {
        all.extend(ec.iter().map(|block| block[i]));
    }

    let mut code = Code { dark: [[false; SIZE]; SIZE], fixed: [[false; SIZE]; SIZE] };
    code.patterns();
    code.place(&all);
    Some(code.dark)
}

fn put(into: &mut [u8], bit: &mut usize, value: u32, count: u32) {
    for i in (0..count).rev() {
        if (value >> i) & 1 == 1 {
            into[*bit >> 3] |= 0x80 >> (*bit & 7);
        }
        *bit += 1;
    }
}

// Reed-Solomon over GF(256) with the polynomial x^8 + x^4 + x^3 + x^2 + 1.

fn multiply(x: u8, y: u8) -> u8 {
    let mut z: u32 = 0;
    for i in (0..8).rev() {
        z = (z << 1) ^ ((z >> 7) * 0x11D);
        z ^= u32::from((y >> i) & 1) * u32::from(x);
    }
    z as u8
}

fn generator(degree: usize) -> Vec<u8> {
    let mut result = vec![0u8; degree];
    result[degree - 1] = 1;
    let mut root = 1u8;
    for _ in 0..degree {
        for j in 0..degree {
            result[j] = multiply(result[j], root);
            if j + 1 < degree {
                result[j] ^= result[j + 1];
            }
        }
        root = multiply(root, 2);
    }
    result
}

fn remainder(data: &[u8], generator: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; generator.len()];
    for byte in data {
        let factor = byte ^ result[0];
        result.rotate_left(1);
        *result.last_mut().expect("a generator of degree 1 or more") = 0;
        for (r, g) in result.iter_mut().zip(generator) {
            *r ^= multiply(*g, factor);
        }
    }
    result
}

struct Code {
    dark: Modules,
    /// The modules of the fixed patterns, which the data does not use.
    fixed: Modules,
}

impl Code {
    fn set(&mut self, x: usize, y: usize, on: bool) {
        self.dark[y][x] = on;
        self.fixed[y][x] = true;
    }

    fn patterns(&mut self) {
        for i in 0..SIZE {
            self.set(6, i, i % 2 == 0);
            self.set(i, 6, i % 2 == 0);
        }
        for (cx, cy) in [(3i32, 3i32), (SIZE as i32 - 4, 3), (3, SIZE as i32 - 4)] {
            for dy in -4i32..=4 {
                for dx in -4i32..=4 {
                    let (x, y, ring) = (cx + dx, cy + dy, dx.abs().max(dy.abs()));
                    if (0..SIZE as i32).contains(&x) && (0..SIZE as i32).contains(&y) {
                        self.set(x as usize, y as usize, ring != 2 && ring != 4);
                    }
                }
            }
        }
        // The one alignment pattern of version 6, centred on module (34, 34).
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                self.set((34 + dx) as usize, (34 + dy) as usize, dx.abs().max(dy.abs()) != 1);
            }
        }
        // The format bits: level Q (binary 11) and mask 0, with their 10 check bits, xored with the fixed pattern.
        let format: u32 = 3 << 3;
        let mut rem = format;
        for _ in 0..10 {
            rem = (rem << 1) ^ ((rem >> 9) * 0x537);
        }
        let bits = (format << 10 | rem) ^ 0x5412;
        let bit = |i: usize| (bits >> i) & 1 == 1;
        for i in 0..=5 {
            self.set(8, i, bit(i));
        }
        self.set(8, 7, bit(6));
        self.set(8, 8, bit(7));
        self.set(7, 8, bit(8));
        for i in 9..15 {
            self.set(14 - i, 8, bit(i));
        }
        for i in 0..8 {
            self.set(SIZE - 1 - i, 8, bit(i));
        }
        for i in 8..15 {
            self.set(8, SIZE - 15 + i, bit(i));
        }
        self.set(8, SIZE - 8, true);
    }

    /// Writes the bytes in the zigzag of the standard, two columns at a time from the right, and applies mask 0.
    fn place(&mut self, all: &[u8]) {
        let mut i = 0;
        let mut right = SIZE - 1;
        loop {
            if right == 6 {
                right = 5;
            }
            for vert in 0..SIZE {
                for j in 0..2 {
                    let x = right - j;
                    let upward = (right + 1) & 2 == 0;
                    let y = if upward { SIZE - 1 - vert } else { vert };
                    if self.fixed[y][x] {
                        continue;
                    }
                    let on = i < all.len() * 8 && (all[i >> 3] >> (7 - (i & 7))) & 1 == 1;
                    i += 1;
                    self.dark[y][x] = on ^ (x + y).is_multiple_of(2);
                }
            }
            if right < 2 {
                break;
            }
            right -= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The reference was drawn by an independent implementation and read back by a QR decoder.
    #[test]
    fn encode_matches_the_reference_code() {
        let text = format!("neb1{}", "q".repeat(62));
        let code = encode(&text).expect("66 bytes fit");
        let drawn: Vec<String> =
            code.iter().map(|row| row.iter().map(|d| if *d { '#' } else { '.' }).collect()).collect();
        let reference: Vec<&str> = include_str!("qr_golden.txt").lines().map(str::trim).collect();
        assert_eq!(drawn, reference);
    }

    #[test]
    fn encode_refuses_a_text_that_does_not_fit() {
        assert!(encode(&"a".repeat(74)).is_some());
        assert!(encode(&"a".repeat(75)).is_none());
    }
}

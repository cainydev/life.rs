pub const CHUNK_SIZE: i32 = 64;

#[derive(Clone, Debug)]
pub struct BitChunk {
    pub data: [u64; 64],
}

impl BitChunk {
    pub fn new() -> Self {
        Self { data: [0; 64] }
    }

    #[inline(always)]
    pub fn set(&mut self, x: i32, y: i32, value: bool) {
        if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= CHUNK_SIZE {
            return;
        }
        if value {
            self.data[y as usize] |= 1 << x;
        } else {
            self.data[y as usize] &= !(1 << x);
        }
    }

    #[inline(always)]
    pub fn get(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= CHUNK_SIZE {
            return false;
        }
        (self.data[y as usize] >> x) & 1 == 1
    }

    // Die magische SIMD Funktion
    pub fn step_bitwise_9(&self, n: [[&BitChunk; 3]; 3]) -> (BitChunk, bool) {
        let mut next_data = [0u64; 64];
        let mut any_alive = false;

        // Referenzen für Lesbarkeit
        let center = &self.data;
        let north = &n[2][1].data;
        let south = &n[0][1].data;
        let west = &n[1][0].data;
        let east = &n[1][2].data;

        // Ecken-Bits (nur 1 Bit pro Ecke relevant!)
        let _nw_bit = n[2][0].data[0] >> 63 & 1; // Von Top-Left Chunk das Bottom-Right Pixel
        let _ne_bit = n[2][2].data[0] & 1; // Von Top-Right Chunk das Bottom-Left Pixel
        let _sw_bit = n[0][0].data[63] >> 63 & 1;
        let _se_bit = n[0][2].data[63] & 1;

        for y in 0..64 {
            let row = center[y];

            // --- Vertikale Nachbarn ---
            // Wenn y=63 (Oben), nehme Zeile 0 vom Nord-Chunk. Sonst Zeile y+1.
            let u = if y == 63 { north[0] } else { center[y + 1] };
            let d = if y == 0 { south[63] } else { center[y - 1] };

            // --- Horizontale Bits für diese Zeile ---
            // Bit ganz links vom East Chunk (Zeile y)
            let e_bit = east[y] & 1;
            // Bit ganz rechts vom West Chunk (Zeile y)
            let w_bit = (west[y] >> 63) & 1;

            // Shift (Left = Richtung x+1, Right = Richtung x-1 in Bit-Logic?)
            // Bevy BitChunk: x=0 ist LSB (1). x=1 ist (2).
            // << 1 schiebt x=0 nach x=1. Das ist "Rechts" im Grid (x+1).
            // >> 1 schiebt x=1 nach x=0. Das ist "Links" im Grid (x-1).

            // Nachbar Rechts (x+1): (row >> 1) | (e_bit << 63)
            let r = (row >> 1) | (e_bit << 63);

            // Nachbar Links (x-1): (row << 1) | w_bit
            let l = (row << 1) | w_bit;

            // --- Diagonale Nachbarn ---
            // Wir müssen das gleiche Shifting für u (Up) und d (Down) machen.
            // Aber Achtung an den Ecken!

            // Oben-Links (x-1, y+1):
            // Wir brauchen das Bit links von der Up-Row.
            // Wenn y=63, brauchen wir das Bit von North-West Chunk?
            // Ja, aber North Chunk Zeile 0 hat links den NW-Chunk.
            // Also reicht es, row 'u' zu shiften und das Rand-Bit vom North-West/West Chunk zu holen.

            // Vereinfachung: Wir bauen 'u' und 'd' mit korrekten Rändern.
            // Das Rand-Bit für 'u' Links kommt aus:
            // Wenn y=63: NW-Chunk[0][63]. Wenn y!=63: West-Chunk[y+1][63].
            let w_bit_u = if y == 63 {
                (n[2][0].data[0] >> 63) & 1
            } else {
                (west[y + 1] >> 63) & 1
            };
            let e_bit_u = if y == 63 {
                n[2][2].data[0] & 1
            } else {
                east[y + 1] & 1
            };

            let w_bit_d = if y == 0 {
                (n[0][0].data[63] >> 63) & 1
            } else {
                (west[y - 1] >> 63) & 1
            };
            let e_bit_d = if y == 0 {
                n[0][2].data[63] & 1
            } else {
                east[y - 1] & 1
            };

            let ul = (u << 1) | w_bit_u;
            let ur = (u >> 1) | (e_bit_u << 63);

            let dl = (d << 1) | w_bit_d;
            let dr = (d >> 1) | (e_bit_d << 63);

            // --- Adder Logic (wie zuvor) ---
            // Inputs: l, r, u, d, ul, ur, dl, dr
            let mut ones = 0u64;
            let mut twos = 0u64;
            let mut fours = 0u64;

            for input in [l, r, u, d, ul, ur, dl, dr] {
                let mask = ones & input;
                ones ^= input;
                let mask2 = twos & mask;
                twos ^= mask;
                fours |= mask2;
            }

            let is_three = ones & twos & !fours;
            let is_two = !ones & twos & !fours;
            let result = is_three | (row & is_two);

            if result != 0 {
                any_alive = true;
            }
            next_data[y] = result;
        }

        (BitChunk { data: next_data }, any_alive)
    }
}

impl Default for BitChunk {
    fn default() -> Self {
        Self::new()
    }
}

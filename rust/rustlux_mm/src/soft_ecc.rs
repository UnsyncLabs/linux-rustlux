// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// soft_ecc.rs — Hamming(72,64) SECDED core for software memory scrubbing
//
// Protects one 64-bit word with an 8-bit shadow code: Single Error
// Correct, Double Error Detect. Construction:
//
//   - a 71-position codeword (positions 1..=71): parity bits at the
//     powers of two (1,2,4,8,16,32,64), the 64 data bits filling the
//     remaining positions in order
//   - each parity bit covers the positions whose index has that bit
//     set, so the syndrome of a single flip IS its position
//   - one extra overall-parity bit (bit 7 of the code byte)
//     distinguishes single flips (parity breaks) from double flips
//     (parity holds) — the "DED" in SECDED
//
// Pure functions, no state: safe to call from scrubber context.
// Mirrored by the C fallback in mm/rustlux_softecc.c
// (patch 0009-rustlux-soft-ecc.patch).

/// number of positional codeword bits (64 data + 7 parity)
const POSITIONS: u32 = 71;

/// outcome of verifying one protected word
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ScrubOutcome {
    /// word and code are consistent
    Clean,
    /// exactly one bit flipped; carries the corrected data word
    Corrected(u64),
    /// two or more bits flipped — beyond SECDED, data is lost
    Uncorrectable,
}

/// FFI mirror of ScrubOutcome for the C scrubber
#[repr(C)]
pub struct SoftEccResult {
    /// 0 = clean, 1 = corrected, -1 = uncorrectable
    pub status: i32,
    /// corrected data word (valid when status == 1)
    pub data: u64,
}

fn is_parity_pos(pos: u32) -> bool {
    pos & (pos - 1) == 0
}

/// XOR of the codeword positions of every set data bit
fn data_syndrome(data: u64) -> u32 {
    let mut syn = 0u32;
    let mut bit = 0u32;
    for pos in 1..=POSITIONS {
        if is_parity_pos(pos) {
            continue;
        }
        if (data >> bit) & 1 == 1 {
            syn ^= pos;
        }
        bit += 1;
    }
    syn
}

/// maps a codeword position back to its data bit index
fn data_bit_index(err_pos: u32) -> u32 {
    let mut idx = 0u32;
    for pos in 1..err_pos {
        if !is_parity_pos(pos) {
            idx += 1;
        }
    }
    idx
}

/// computes the 8-bit SECDED code for a 64-bit word
pub fn encode(data: u64) -> u8 {
    // parity bit i (position 2^i) must cancel bit i of the data
    // syndrome, so the low 7 bits of the code ARE the syndrome
    let mut code = (data_syndrome(data) & 0x7f) as u8;

    // overall parity over data + positional parity bits, stored so
    // the total number of ones (including this bit) is even
    if (data.count_ones() + (code as u32).count_ones()) & 1 == 1 {
        code |= 0x80;
    }
    code
}

/// verifies a word against its code, correcting single-bit flips
pub fn verify(data: u64, code: u8) -> ScrubOutcome {
    // XOR of distinct powers of two == the integer value of the low
    // 7 code bits, so the parity contribution to the syndrome is just
    // (code & 0x7f)
    let syn = data_syndrome(data) ^ (code & 0x7f) as u32;
    let parity_ok = (data.count_ones() + (code as u32).count_ones()) & 1 == 0;

    match (syn, parity_ok) {
        (0, true) => ScrubOutcome::Clean,
        // only the overall parity bit flipped; data intact
        (0, false) => ScrubOutcome::Corrected(data),
        // an even number of flips keeps overall parity: beyond SECDED
        (_, true) => ScrubOutcome::Uncorrectable,
        (pos, false) => {
            if pos > POSITIONS {
                // impossible position: multi-bit corruption
                ScrubOutcome::Uncorrectable
            } else if is_parity_pos(pos) {
                // a shadow parity bit flipped; data intact
                ScrubOutcome::Corrected(data)
            } else {
                ScrubOutcome::Corrected(data ^ (1u64 << data_bit_index(pos)))
            }
        }
    }
}

/// FFI entry point: computes the SECDED code for one word
#[no_mangle]
pub extern "C" fn rustlux_softecc_encode(data: u64) -> u8 {
    encode(data)
}

/// FFI entry point: verifies one word, correcting single-bit flips
#[no_mangle]
pub extern "C" fn rustlux_softecc_verify(data: u64, code: u8) -> SoftEccResult {
    match verify(data, code) {
        ScrubOutcome::Clean => SoftEccResult { status: 0, data },
        ScrubOutcome::Corrected(fixed) => SoftEccResult { status: 1, data: fixed },
        ScrubOutcome::Uncorrectable => SoftEccResult { status: -1, data },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLES: [u64; 8] = [
        0,
        u64::MAX,
        0xAA55_AA55_AA55_AA55,
        0xDEAD_BEEF_CAFE_BABE,
        1,
        1 << 63,
        0x0123_4567_89AB_CDEF,
        0xFFFF_0000_FFFF_0000,
    ];

    #[test]
    fn clean_roundtrip() {
        for &d in &SAMPLES {
            assert_eq!(verify(d, encode(d)), ScrubOutcome::Clean);
        }
    }

    #[test]
    fn every_data_bit_flip_is_corrected() {
        for &d in &SAMPLES {
            let code = encode(d);
            for bit in 0..64 {
                let corrupted = d ^ (1u64 << bit);
                assert_eq!(
                    verify(corrupted, code),
                    ScrubOutcome::Corrected(d),
                    "data bit {bit} of {d:#x} must correct back"
                );
            }
        }
    }

    #[test]
    fn every_code_bit_flip_leaves_data_intact() {
        for &d in &SAMPLES {
            let code = encode(d);
            for bit in 0..8 {
                let bad_code = code ^ (1u8 << bit);
                assert_eq!(
                    verify(d, bad_code),
                    ScrubOutcome::Corrected(d),
                    "code bit {bit} flip must not touch the data"
                );
            }
        }
    }

    #[test]
    fn double_data_flip_is_uncorrectable() {
        for &d in &SAMPLES {
            let code = encode(d);
            assert_eq!(verify(d ^ 0b11, code), ScrubOutcome::Uncorrectable);
            assert_eq!(
                verify(d ^ (1 << 3) ^ (1 << 40), code),
                ScrubOutcome::Uncorrectable
            );
        }
    }

    #[test]
    fn data_plus_code_double_flip_is_uncorrectable() {
        let d = 0xDEAD_BEEF_CAFE_BABE_u64;
        let code = encode(d);
        // one flip in data plus one in the shadow code = double error
        assert_eq!(
            verify(d ^ (1 << 10), code ^ 0b100),
            ScrubOutcome::Uncorrectable
        );
    }

    #[test]
    fn rowhammer_scenario_page_table_bit() {
        // a PTE-like word where an attacker flips one bit (e.g. to
        // gain write access): the scrubber must restore the original
        let pte = 0x8000_0000_1234_5067_u64;
        let code = encode(pte);
        let hammered = pte ^ (1 << 1); // flip the RW bit
        assert_eq!(verify(hammered, code), ScrubOutcome::Corrected(pte));
    }

    #[test]
    fn ffi_status_codes() {
        let d = 0xAA55_AA55_AA55_AA55_u64;
        let code = rustlux_softecc_encode(d);

        let clean = rustlux_softecc_verify(d, code);
        assert_eq!(clean.status, 0);

        let fixed = rustlux_softecc_verify(d ^ (1 << 17), code);
        assert_eq!(fixed.status, 1);
        assert_eq!(fixed.data, d);

        let lost = rustlux_softecc_verify(d ^ 0b11, code);
        assert_eq!(lost.status, -1);
    }
}

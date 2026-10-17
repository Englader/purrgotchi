//! Packs a `PxCanvas` into Claude Code `Raster` cells for the cat band.
//!
//! Each cell is three little-endian u32: `[codepoint, fg, bg]`. Colours are
//! snapped to multiples of 0x11 per channel (keeps the distinct-pair count
//! under the Raster limit); `DEFAULT_COLOR` is the terminal default.

use super::PxCanvas;
use crate::colors::Rgb;

/// Raster "terminal default" colour.
pub(crate) const DEFAULT_COLOR: u32 = 0x0100_0000;

/// Each channel to the nearest multiple of 17 (0x11), packed `0x00RRGGBB`.
pub(crate) fn snap(c: Rgb) -> u32 {
    let s = |v: u8| u32::from(v).saturating_add(8) / 17 * 17;
    s(c.0) << 16 | s(c.1) << 8 | s(c.2)
}

/// `rows * width` cells, row-major; terminal row `r` carries pixel rows
/// `2r` (top) and `2r + 1` (bottom).
pub(crate) fn pack_cells(c: &PxCanvas, rows: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows * c.width() * 12);
    for r in 0..rows {
        for x in 0..c.width() as i32 {
            let (t, b) = (c.get(x, 2 * r as i32), c.get(x, 2 * r as i32 + 1));
            let cell = match (t, b) {
                (None, None) => [0x20, DEFAULT_COLOR, DEFAULT_COLOR],
                (Some(t), None) => [0x2580, snap(t), DEFAULT_COLOR],
                (None, Some(b)) => [0x2584, snap(b), DEFAULT_COLOR],
                (Some(t), Some(b)) => [0x2580, snap(t), snap(b)],
            };
            for v in cell {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    out
}

/// Standard padded base64 (RFC 4648).
pub(crate) fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for ch in bytes.chunks(3) {
        let n = (u32::from(ch[0]) << 16) | (u32::from(*ch.get(1).unwrap_or(&0)) << 8) | u32::from(*ch.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= ch.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(b: &[u8]) -> Vec<u32> {
        b.chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
    }

    #[test]
    fn base64_rfc4648_vectors() {
        for (i, o) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(i.as_bytes()), o);
        }
    }

    #[test]
    fn snap_rounds_to_0x11() {
        assert_eq!(snap(Rgb(0, 9, 255)), 0x0000_11FF);
        assert_eq!(snap(Rgb(8, 0x88, 0xF7)), 0x0000_88FF);
    }

    #[test]
    fn pack_cells_rules() {
        let (a, b) = (Rgb(0x10, 0x20, 0x30), Rgb(255, 0, 0));
        let mut c = PxCanvas::new(2);
        c.set(0, 0, a);
        c.set(1, 1, b);
        let out = pack_cells(&c, 1);
        assert_eq!(out.len(), 24);
        assert_eq!(cells(&out), [0x2580, snap(a), DEFAULT_COLOR, 0x2584, snap(b), DEFAULT_COLOR]);
        c.set(0, 1, b);
        assert_eq!(cells(&pack_cells(&c, 1))[..3], [0x2580, snap(a), snap(b)]);
    }

    #[test]
    fn empty_canvas_is_terminal_default() {
        let out = pack_cells(&PxCanvas::new(3), 6);
        assert_eq!(out.len(), 3 * 6 * 12);
        assert!(cells(&out).chunks(3).all(|c| c == [0x20, DEFAULT_COLOR, DEFAULT_COLOR]));
    }
}

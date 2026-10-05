//! Image headers, read without a decoding dependency.

/// The pixel dimensions of a PNG or JPEG, read from its header.
///
/// Enough to fill in `width` and `height` on the portrait, and small enough
/// not to be worth a decoding dependency. Anything else — WebP, AVIF, SVG —
/// returns `None`, and the attributes are simply omitted.
pub fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        // IHDR is always the first chunk: width and height at a fixed offset.
        let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
        let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
        return Some((width, height));
    }

    if bytes.starts_with(&[0xff, 0xd8]) {
        // Walk the marker segments to the start-of-frame, which is the only one
        // that states the image size.
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xff {
                return None;
            }
            let marker = bytes[i + 1];
            // Fill bytes, and the standalone markers that carry no length.
            if marker == 0xff {
                i += 1;
                continue;
            }
            if matches!(marker, 0x01 | 0xd0..=0xd9) {
                i += 2;
                continue;
            }
            let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            let start_of_frame =
                matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf);
            if start_of_frame {
                let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                return Some((width, height));
            }
            i += 2 + length;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_png_and_jpeg_dimensions() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        png.extend_from_slice(&[0, 0, 0, 13]);
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        assert_eq!(dimensions(&png), Some((640, 480)));

        // SOI, an APP0 segment to skip past, then SOF0.
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00];
        jpeg.extend_from_slice(&[0xff, 0xc0, 0x00, 0x11, 0x08]);
        jpeg.extend_from_slice(&800u16.to_be_bytes()); // height
        jpeg.extend_from_slice(&600u16.to_be_bytes()); // width
        jpeg.extend_from_slice(&[0u8; 8]);
        assert_eq!(dimensions(&jpeg), Some((600, 800)));

        assert_eq!(dimensions(b"RIFF....WEBP"), None);
    }
}

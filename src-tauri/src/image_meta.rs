pub fn to_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

pub fn is_png(bytes: &[u8]) -> bool {
    const PNG_SIG: &[u8] = b"\x89PNG\r\n\x1a\n";
    bytes.len() >= PNG_SIG.len() && &bytes[..PNG_SIG.len()] == PNG_SIG
}

pub fn embed_png_itxt(bytes: &[u8], key: &str, value: &str) -> Vec<u8> {
    if !is_png(bytes) {
        return bytes.to_vec();
    }

    let iend_pos = find_iend(bytes);

    let keyword = key.as_bytes();
    let text = value.as_bytes();
    let mut data: Vec<u8> = Vec::with_capacity(keyword.len() + 5 + text.len());
    data.extend_from_slice(keyword);
    data.push(0);
    data.push(0); // compression flag: uncompressed
    data.push(0); // compression method: zlib, ignored when uncompressed
    data.push(0); // language tag: empty
    data.push(0); // translated keyword: empty
    data.extend_from_slice(text);

    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(b"iTXt");
    crc_input.extend_from_slice(&data);
    let checksum = crc32(&crc_input);

    let mut chunk = Vec::with_capacity(12 + data.len());
    chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
    chunk.extend_from_slice(b"iTXt");
    chunk.extend_from_slice(&data);
    chunk.extend_from_slice(&checksum.to_be_bytes());

    let mut result = Vec::with_capacity(bytes.len() + chunk.len());
    result.extend_from_slice(&bytes[..iend_pos]);
    result.extend_from_slice(&chunk);
    result.extend_from_slice(&bytes[iend_pos..]);
    result
}

fn find_iend(bytes: &[u8]) -> usize {
    let mut pos = 8;
    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap_or([0; 4])) as usize;
        if &bytes[pos + 4..pos + 8] == b"IEND" {
            return pos;
        }
        pos += 12 + len;
    }
    bytes.len()
}

fn crc32(data: &[u8]) -> u32 {
    let table = make_crc32_table();
    let mut crc: u32 = 0xffffffff;
    for &byte in data {
        crc = table[((crc ^ byte as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffffffff
}

fn make_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for i in 0..256 {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb88320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        table[i] = c;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_iend_known_vector() {
        assert_eq!(crc32(b"IEND"), 0xAE426082);
    }

    #[test]
    fn embed_inserts_itxt_chunk_before_iend() {
        let png = minimal_png();
        let result = embed_png_itxt(&png, "hello", "world");
        assert!(result.len() > png.len());
        let text_pos = result.windows(4).position(|w| w == b"iTXt");
        let iend_pos = result.windows(4).position(|w| w == b"IEND");
        assert!(text_pos.is_some() && iend_pos.is_some());
        assert!(text_pos.unwrap() < iend_pos.unwrap());
    }

    #[test]
    fn embed_preserves_multiple_itxt_chunks() {
        let png = minimal_png();
        let with_prompt = embed_png_itxt(&png, "prompt", "rendered prompt");
        let result = embed_png_itxt(&with_prompt, "sozocraft", r#"{"schemaVersion":1}"#);
        let chunks = itxt_chunks(&result);

        assert!(chunks.contains(&("prompt".to_string(), "rendered prompt".to_string())));
        assert!(chunks.contains(&(
            "sozocraft".to_string(),
            r#"{"schemaVersion":1}"#.to_string()
        )));
    }

    #[test]
    fn embed_itxt_preserves_utf8_text() {
        let png = minimal_png();
        let result = embed_png_itxt(&png, "prompt", "中文 prompt");
        let chunks = itxt_chunks(&result);

        assert!(chunks.contains(&("prompt".to_string(), "中文 prompt".to_string())));
    }

    #[test]
    fn non_png_returned_unchanged() {
        let jpeg = b"\xff\xd8\xff\xe0hello world";
        let result = embed_png_itxt(jpeg, "key", "value");
        assert_eq!(result, jpeg.to_vec());
    }

    #[test]
    fn is_png_checks_signature() {
        assert!(is_png(&minimal_png()));
        assert!(!is_png(b"\xff\xd8\xff\xe0hello world"));
    }

    fn minimal_png() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        // IHDR chunk (fake data, 4 bytes)
        let ihdr_data = [0u8; 4];
        let mut ihdr_crc_input = Vec::new();
        ihdr_crc_input.extend_from_slice(b"IHDR");
        ihdr_crc_input.extend_from_slice(&ihdr_data);
        buf.extend_from_slice(&(ihdr_data.len() as u32).to_be_bytes());
        buf.extend_from_slice(b"IHDR");
        buf.extend_from_slice(&ihdr_data);
        buf.extend_from_slice(&crc32(&ihdr_crc_input).to_be_bytes());
        // IEND chunk
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(b"IEND");
        buf.extend_from_slice(&crc32(b"IEND").to_be_bytes());
        buf
    }

    fn itxt_chunks(bytes: &[u8]) -> Vec<(String, String)> {
        let mut chunks = Vec::new();
        let mut pos = 8;
        while pos + 12 <= bytes.len() {
            let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
            let chunk_type = &bytes[pos + 4..pos + 8];
            let data_start = pos + 8;
            let data_end = data_start + len;
            if data_end > bytes.len() {
                break;
            }
            if chunk_type == b"iTXt" {
                let data = &bytes[data_start..data_end];
                if let Some(split) = data.iter().position(|byte| *byte == 0) {
                    let text_start = split + 5;
                    if text_start > data.len() {
                        pos += 12 + len;
                        continue;
                    }
                    let key = String::from_utf8_lossy(&data[..split]);
                    let value = String::from_utf8_lossy(&data[text_start..]);
                    chunks.push((key.to_string(), value.to_string()));
                }
            }
            pos += 12 + len;
        }
        chunks
    }
}

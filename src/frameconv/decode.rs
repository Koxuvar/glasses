pub fn decode_hap_packet(data: &[u8]) -> Vec<u8> {
    if data.len() < 4 {
        return data.to_vec();
    }

    let section_size = u32::from_le_bytes([data[0], data[1], data[2], 0]) as usize;
    let section_type = data[3];
    let payload = &data[4..4 + section_size.min(data.len() - 4)];

    match section_type {
        0xAB => payload.to_vec(),
        0xBB => snap::raw::Decoder::new()
            .decompress_vec(payload)
            .unwrap_or_else(|_| payload.to_vec()),
        0xCB => decode_hap_instructions(payload),
        0xAE => payload.to_vec(),
        0xBE => snap::raw::Decoder::new()
            .decompress_vec(payload)
            .unwrap_or_else(|_| payload.to_vec()),
        0xCE => decode_hap_instructions(payload),
        0xAF => payload.to_vec(),
        0xBF => snap::raw::Decoder::new()
            .decompress_vec(payload)
            .unwrap_or_else(|_| payload.to_vec()),
        0xCF => decode_hap_instructions(payload),
        _ => payload.to_vec(),
    }
}

fn decode_hap_instructions(data: &[u8]) -> Vec<u8> {
    if data.len() < 4 {
        return data.to_vec();
    }

    let inner_size = u32::from_le_bytes([data[0], data[1], data[2], 0]) as usize;
    let inner_type = data[3];

    if inner_type != 0x01 {
        return data.to_vec();
    }

    let instructions_end = 4 + inner_size;
    if instructions_end > data.len() {
        return data.to_vec();
    }

    let instructions = &data[4..instructions_end];
    let frame_data = &data[instructions_end..];

    let mut compressors: Vec<u8> = Vec::new();
    let mut chunk_sizes: Vec<u32> = Vec::new();
    let mut offset = 0;

    while offset + 4 <= instructions.len() {
        let size = u32::from_le_bytes([
            instructions[offset],
            instructions[offset + 1],
            instructions[offset + 2],
            0,
        ]) as usize;
        let section_type = instructions[offset + 3];

        if offset + 4 + size > instructions.len() {
            break;
        }

        let section_data = &instructions[offset + 4..offset + 4 + size];

        match section_type {
            0x02 => compressors = section_data.to_vec(),
            0x03 => {
                chunk_sizes = section_data
                    .chunks(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect()
            }
            _ => {}
        }
        offset += 4 + size;
    }

    if chunk_sizes.is_empty() {
        return frame_data.to_vec();
    }

    let mut result = Vec::new();
    let mut chunk_offset = 0;

    for (i, &chunk_size) in chunk_sizes.iter().enumerate() {
        let end = chunk_offset + chunk_size as usize;
        if end > frame_data.len() {
            break;
        }
        let chunk = &frame_data[chunk_offset..end];
        let decompressed = if i < compressors.len() {
            match compressors[i] {
                0x0A => chunk.to_vec(),
                0x0B => snap::raw::Decoder::new()
                    .decompress_vec(chunk)
                    .unwrap_or_else(|_| chunk.to_vec()),
                _ => chunk.to_vec(),
            }
        } else {
            chunk.to_vec()
        };
        result.extend_from_slice(&decompressed);
        chunk_offset = end;
    }

    result
}

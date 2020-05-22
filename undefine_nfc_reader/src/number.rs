pub fn to_u16_big_endian(array: &[u8]) -> Option<u16> {
    let first_part = array.get(0)?;
    let second_part = array.get(1)?;
    Some((first_part.clone() << 8) as u16 + second_part.clone() as u16)
}

pub fn get_bytes_big_endian(value: u16) -> Vec<u8> {
    vec![(value >> 8) as u8, (value & 255) as u8]
}

pub fn calculate_length_field(byte_size: usize) -> Vec<u8> {
    if byte_size + 1 <= 0x7F {
        vec![(byte_size + 1) as u8]
    } else if byte_size + 2 <= 0xFF {
        vec![0x81, (byte_size + 2) as u8]
    } else if byte_size + 3 <= 0xFFFF {
        let mut vec: Vec<u8> = vec![0x82];
        vec.append(&mut get_bytes_big_endian(byte_size as u16));
        vec
    } else {
        panic!("incorrect message size");
    }
}

pub fn calculate_lrc(buf: &[u8]) -> u8 {
    let mut lrc: u8 = 0;
    for b in buf {
        lrc ^= b;
    }

    return lrc;
}

pub fn get_message_length(buf: &[u8], offset: usize) -> Option<(u16, usize)> {
    if buf[offset] == 0x81 {
        Some((buf[offset + 1] as u16, offset + 2))
    } else if buf[offset] == 0x82 {
        Some((
            to_u16_big_endian(&buf[(offset + 1)..=(offset + 3)])?,
            offset + 3,
        ))
    } else {
        Some((buf[offset] as u16, offset + 1))
    }
}

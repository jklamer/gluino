use std::{
    io::{self, Read},
    mem::size_of,
};

pub fn variable_length_encode(mut z: usize, buffer: &mut Vec<u8>) {
    loop {
        if z <= 0x7F {
            buffer.push((z & 0x7F) as u8);
            break;
        } else {
            buffer.push((0x80 | (z & 0x7F)) as u8);
            z >>= 7;
        }
    }
}
const SYSTEM_SIZE: usize = size_of::<usize>();
const MAX_SYSTEM_BYTES: usize = 16;

pub trait VariableLengthDecodingTarget {
    const BYTE_LEN: usize;
    fn from_le_bytes(b: &[u8]) -> Self;
}

macro_rules! gen_vldt_impls_nums {
    ($($T:ty)+) => {
        $(
        impl VariableLengthDecodingTarget for $T {
            const BYTE_LEN: usize = std::mem::size_of::<$T>();
        
            #[inline]
            fn from_le_bytes(b: &[u8]) -> Self {
                assert!(b.len() >= Self::BYTE_LEN);
                let mut tmp : [u8; Self::BYTE_LEN] = [0; Self::BYTE_LEN];
                tmp.copy_from_slice(&b[0..Self::BYTE_LEN]);
                Self::from_le_bytes(tmp)
            }
        }
        )+
    }
}
gen_vldt_impls_nums!(u8 u16 u32 u64 u128 usize);

#[derive(Debug)]
pub enum VariableLengthResult<B: VariableLengthDecodingTarget> {
    Respresentable(B),
    Unrepresentable(Vec<u8>), // litte indian arbitrary length unint
}

pub fn variable_lenth_decode<R: Read, B: VariableLengthDecodingTarget>(
    input: &mut R,
) -> Result<VariableLengthResult<B>, VariableLengthDecodingError> {
    let mut read_buffer = [0u8; MAX_SYSTEM_BYTES];
    let mut tracking_index = 0usize;
    let mut overflow = Vec::<u8>::with_capacity(0);
    let mut shift_offset = 0usize;
    loop {
        if tracking_index >= B::BYTE_LEN {
            overflow.copy_from_slice(&read_buffer[0..(B::BYTE_LEN - 1)]);
            tracking_index = 1;
            read_buffer[0] = read_buffer[B::BYTE_LEN - 1];
        }
        let read_result = input.read_exact(&mut read_buffer[tracking_index..tracking_index + 1]);
        match read_result {
            Ok(()) => {
                let last_byte = read_buffer[tracking_index] & 0x80 == 0;
                read_buffer[tracking_index] &= 0x7F;
                if tracking_index > 0 {
                    //compact bytes
                    read_buffer[tracking_index - 1] |=
                        read_buffer[tracking_index] << (8 - shift_offset & 0x07);
                    read_buffer[tracking_index] >>= shift_offset & 0x07;
                }
                if last_byte {
                    break;
                }
                shift_offset += 1;
                if shift_offset >= 8 && shift_offset & 0x07 == 0 {
                    tracking_index -= 1;
                }
                tracking_index += 1;
            }
            Err(e) => {
                return Err(match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => {
                        VariableLengthDecodingError::IncompleteVariableLengthEncoding
                    }
                    _ => VariableLengthDecodingError::IoError(e),
                })
            }
        }
    }
    if overflow.is_empty() {
        Ok(VariableLengthResult::Respresentable(B::from_le_bytes(
            &read_buffer,
        )))
    } else {
        overflow.copy_from_slice(&read_buffer);
        Ok(VariableLengthResult::Unrepresentable(overflow))
    }
}

#[derive(Debug)]
pub enum VariableLengthDecodingError {
    IncompleteVariableLengthEncoding,
    IoError(io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_veriable_encoding() {
        let mut out = vec![];
        let buffer = &mut out;
        variable_length_encode(0, buffer);
        variable_length_encode(127, buffer);
        variable_length_encode(128, buffer);
        variable_length_encode(16383, buffer);
        variable_length_encode(16384, buffer);
        variable_length_encode(2097151, buffer);
        variable_length_encode(2097152, buffer);
        variable_length_encode(268435455, buffer);
        variable_length_encode(268435456, buffer);

        let mut out = &out[..];
        //let r = variable_lenth_decode(&mut out).unwrap();
        assert!(matches!(
            variable_lenth_decode(&mut out).unwrap(),
            VariableLengthResult::Respresentable(0usize)
        ));
        assert!(matches!(
            variable_lenth_decode(&mut out).unwrap(),
            VariableLengthResult::Respresentable(127usize)
        ));
        assert!(matches!(
            variable_lenth_decode(&mut out).unwrap(),
            VariableLengthResult::Respresentable(128u128)
        ));
        assert!(matches!(
            variable_lenth_decode(&mut out).unwrap(),
            VariableLengthResult::Respresentable(16383u64)
        ));
        assert!(matches!(
            variable_lenth_decode(&mut out).unwrap(),
            VariableLengthResult::Respresentable(16384usize),
        ));
        assert!(matches!(
            variable_lenth_decode(&mut out).unwrap(),
            VariableLengthResult::Respresentable(2097151u128)
        ));
        assert!(matches!(
            variable_lenth_decode(&mut out).unwrap(),
            VariableLengthResult::Respresentable(2097152u64)
        ));

        // println!("{:?}", variable_lenth_decode(&[0xFF, 0xFF, 0x7F]));
        // println!("{:?}", variable_lenth_decode(&[0x80, 0x80, 0x80, 0x01]));

        // println!("{:?}", variable_lenth_decode(&mut [0xFF, 0xFF, 0xFF, 0x7F]));
        // println!(
        //     "{:?}",
        //     variable_lenth_decode(&mut [0x80, 0x80, 0x80, 0x80, 0x01])
        // );
    }
}

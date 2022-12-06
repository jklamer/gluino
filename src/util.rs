use std::io::{self, Read, Write};

pub trait WriteAllReturnSize {
    fn write_all_size(&mut self, bytes: &[u8]) -> Result<usize, io::Error>;
}

impl<W: Write> WriteAllReturnSize for W {
    #[inline]
    fn write_all_size(&mut self, bytes: &[u8]) -> Result<usize, io::Error> {
        self.write_all(bytes)?;
        Ok(bytes.len())
    }
}

const MAX_LANG_BYTES: usize = 16;
const MAX_SYSTEM_BYTES_VLE: usize = MAX_LANG_BYTES * 8 / 7 + 1;

pub fn variable_length_encode_u64<W: Write>(mut z: u64, out: &mut W) -> Result<usize, io::Error> {
    let mut encoding = [0u8; MAX_SYSTEM_BYTES_VLE];
    let mut n = 0usize;
    while z > 0x7F {
        encoding[n] = (0x80 | (z & 0x7F)) as u8;
        z >>= 7;
        n += 1;
    }
    encoding[n] = (z & 0x7F) as u8;
    out.write_all_size(&encoding[0..=n])
}
pub fn variable_length_decode_u64<R: Read>(
    input: &mut R,
) -> Result<VariableLengthResult<u64>, VariableLengthDecodingError> {
    variable_lenth_decode(input)
}

pub fn variable_length_encode_u128<W: Write>(mut z: u128, out: &mut W) -> Result<usize, io::Error> {
    let mut encoding = [0u8; MAX_SYSTEM_BYTES_VLE];
    let mut n = 0usize;
    while z > 0x7F {
        encoding[n] = (0x80 | (z & 0x7F)) as u8;
        z >>= 7;
        n += 1;
    }
    encoding[n] = (z & 0x7F) as u8;
    out.write_all_size(&encoding[0..=n])
}

pub fn variable_length_decode_u128<R: Read>(
    input: &mut R,
) -> Result<VariableLengthResult<u128>, VariableLengthDecodingError> {
    variable_lenth_decode(input)
}

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
gen_vldt_impls_nums!(u16 u32 u64 u128 usize);

#[derive(Debug)]
pub enum VariableLengthResult<B: VariableLengthDecodingTarget> {
    Respresentable(B),
    Unrepresentable(Vec<u8>), // litte indian arbitrary length uint
}

pub fn variable_lenth_decode<R: Read, B: VariableLengthDecodingTarget>(
    input: &mut R,
) -> Result<VariableLengthResult<B>, VariableLengthDecodingError> {
    let mut read_buffer = [0u8; MAX_LANG_BYTES];
    let mut tracking_index = 0usize;
    let mut overflow = Vec::<u8>::with_capacity(0);
    let mut shift_offset = 0usize;
    let mut last_byte_reached: bool = false;
    while !last_byte_reached {
        if tracking_index >= B::BYTE_LEN {
            overflow.extend_from_slice(&read_buffer[0..(B::BYTE_LEN - 1)]);
            tracking_index = 1;
            read_buffer[0] = read_buffer[B::BYTE_LEN - 1];
        }
        let read_result = input.read_exact(&mut read_buffer[tracking_index..tracking_index + 1]);
        match read_result {
            Ok(()) => {
                last_byte_reached = read_buffer[tracking_index] & 0x80 == 0;
                read_buffer[tracking_index] &= 0x7F;
                if tracking_index > 0 {
                    //compact bytes
                    read_buffer[tracking_index - 1] |= read_buffer[tracking_index]
                        .checked_shl(8 - (shift_offset as u32 & 0x07))
                        .unwrap_or(0);
                    read_buffer[tracking_index] >>= shift_offset & 0x07;
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
    if overflow.is_empty() && tracking_index < B::BYTE_LEN {
        Ok(VariableLengthResult::Respresentable(B::from_le_bytes(
            &read_buffer,
        )))
    } else {
        overflow.extend_from_slice(&read_buffer[0..tracking_index]);
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

    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_veriable_encoding() {
        let mut out = vec![];
        let buffer = &mut out;
        variable_length_encode_u64(0, buffer).unwrap();
        variable_length_encode_u64(127, buffer).unwrap();
        variable_length_encode_u64(128, buffer).unwrap();
        variable_length_encode_u128(16383, buffer).unwrap();
        variable_length_encode_u128(16384, buffer).unwrap();
        variable_length_encode_u64(2097151, buffer).unwrap();
        variable_length_encode_u128(2097152, buffer).unwrap();
        variable_length_encode_u128(268435455, buffer).unwrap();
        variable_length_encode_u64(268435456, buffer).unwrap();

        let mut out = &out[..];
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
    }

    #[test]
    fn test_unrepresentable_decoding() {
        let mut out = vec![];
        let buffer = &mut out;
        variable_length_encode_u64(268435455, buffer).unwrap();
        variable_length_encode_u128(268435456, buffer).unwrap();
        let mut out = &out[..];
        if let VariableLengthResult::<u16>::Unrepresentable(v) =
            variable_lenth_decode(&mut out).unwrap()
        {
            let mut v2 = [0u8; 4];
            v2[0..v.len()].copy_from_slice(&v[..]);
            assert_eq!(268435455, u32::from_le_bytes(v2));
        } else {
            assert!(false);
        }
        if let VariableLengthResult::<u32>::Unrepresentable(v) =
            variable_lenth_decode(&mut out).unwrap()
        {
            let mut v2 = [0u8; 8];
            v2[0..v.len()].copy_from_slice(&v[..]);
            assert_eq!(268435456, u64::from_le_bytes(v2));
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_way_too_big() {
        match variable_length_decode_u128(&mut Cursor::new(&[
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0x01,
        ])) {
            Ok(VariableLengthResult::Respresentable(n)) => {
                assert!(false, "Should overflow")
            }
            Ok(VariableLengthResult::Unrepresentable(_)) => {
                assert!(true);
            }
            Err(e) => {
                assert!(true);
            }
        };
    }
}

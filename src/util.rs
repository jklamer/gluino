use std::io::Read;



pub fn variable_length_encode(mut z: u64, buffer: &mut Vec<u8>) {
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

pub fn variable_lenth_decode(buffer: &[u8]) -> Result<u64,VariableLengthDecodingError> {
    let mut result = 0u64;
    if buffer.len() > 9 {
        Err(VariableLengthDecodingError::TooManyBytes)
    } else {
        for i in 0..buffer.len() {
            let last_byte = i == (buffer.len() - 1);
            let msb_1 = buffer[i] & 0x80 > 0;
            if last_byte && msb_1 {
                return Err(VariableLengthDecodingError::IncompleteVariableLengthEncoding)
            }else if !last_byte && !msb_1 {
                return Err(VariableLengthDecodingError::TooManyBytes)
            }
            result |= u64::from(buffer[i] & 0x7F) << i * 7
        }
        Ok(result)
    } 
}

#[derive(Debug)]
pub enum VariableLengthDecodingError {
    IncompleteVariableLengthEncoding,
    TooManyBytes,
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

        assert_eq!(0,variable_lenth_decode(&out[0..1]).unwrap());
        assert_eq!(127,variable_lenth_decode(&out[1..2]).unwrap());
        assert_eq!(128,variable_lenth_decode(&out[2..4]).unwrap());
        assert_eq!(16383,variable_lenth_decode(&out[4..6]).unwrap());
        assert_eq!(16384,variable_lenth_decode(&out[6..9]).unwrap());
        assert_eq!(2097151,variable_lenth_decode(&out[9..12]).unwrap());
        assert_eq!(2097152,variable_lenth_decode(&out[12..16]).unwrap());

        println!("{:?}", variable_lenth_decode(&[0xFF, 0xFF, 0x7F]));
        println!("{:?}", variable_lenth_decode(&[0x80, 0x80, 0x80, 0x01]));

        println!("{:?}", variable_lenth_decode(&[0xFF, 0xFF, 0xFF, 0x7F]));
        println!("{:?}", variable_lenth_decode(&[0x80, 0x80, 0x80, 0x80, 0x01]));
    }
}
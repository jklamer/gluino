use std::io::{Read, self, Write};

use crate::util::WriteAllReturnSize;

use super::GluinoValueKind;


trait Encodable 
where
    Self: Sized
{
    fn encode<W:Write>(&self, writer: &mut W) -> Result<usize, io::Error>;
    fn decode<R:Read>(reader: &mut R) -> Result<Self, io::Error>;
    fn kind(&self) -> GluinoValueKind;
}

impl Encodable for bool {
    #[inline]
    fn encode<W:Write>(&self, writer: &mut W) -> Result<usize, io::Error> {
        if *self {
            writer.write_all_size(&[1])
        } else {
            writer.write_all_size(&[0])
        }
    }

    #[inline]
    fn decode<R:Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut b = [0u8];
        reader.read_exact(&mut b)?;
        Ok(if b[0] > 0 {
            true
        } else {
            false
        })
    }

    #[inline]
    fn kind(&self) -> GluinoValueKind {
       GluinoValueKind::Bool
    }
}

macro_rules! encode_integer_type {
    ($type:ty, $kind:expr) => {
        impl Encodable for $type {
            fn encode<W:Write>(&self, writer: &mut W) -> Result<usize, io::Error> {
                writer.write_all_size(&self.to_le_bytes())
            }
            fn decode<R:Read>(reader: &mut R) -> Result<Self, io::Error> {
                let mut buff = [0u8; (<$type>::BITS >> 3) as usize];
                reader.read_exact(&mut buff)?;
                Ok(<$type>::from_le_bytes(buff))
            }
            fn kind(&self) -> GluinoValueKind{
                $kind
            }
        }
    }
}

encode_integer_type!(u8, GluinoValueKind::Uint8);
encode_integer_type!(u16, GluinoValueKind::Uint16);
encode_integer_type!(u32, GluinoValueKind::Uint32);
encode_integer_type!(u64, GluinoValueKind::Uint64);
encode_integer_type!(u128, GluinoValueKind::Uint128);
encode_integer_type!(i8, GluinoValueKind::Int8);
encode_integer_type!(i16, GluinoValueKind::Int16);
encode_integer_type!(i32, GluinoValueKind::Int32);
encode_integer_type!(i64, GluinoValueKind::Int64);
encode_integer_type!(i128, GluinoValueKind::Int128);


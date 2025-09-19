use std::io::{self, Read, Write};

use crate::util::WriteAllReturnSize;

use super::{F32, F64, GluinoSerializationError, GluinoValue, GluinoValueKind};

pub trait Encodable
where
    Self: Sized,
{
    fn extract(value: GluinoValue) -> Result<Self, GluinoSerializationError>;
    fn encode<W: Write>(&self, writer: &mut W) -> Result<usize, io::Error>;
    fn decode<R: Read>(reader: &mut R) -> Result<GluinoValue, io::Error>;
}

macro_rules! default_extract {
    ($kind:ident) => {
        #[inline]
        fn extract(value: GluinoValue) -> Result<Self, GluinoSerializationError> {
            if let GluinoValue::$kind(b) = value {
                Ok(b)
            } else {
                Err(GluinoSerializationError::ValueKindMismatch {
                    expected_value_kind: GluinoValueKind::$kind,
                    actual_value_kind: value.into(),
                })
            }
        }
    };
}

macro_rules! encode_integer_type {
    ($type:ty, $kind:ident) => {
        impl Encodable for $type {
            #[inline]
            fn encode<W: Write>(&self, writer: &mut W) -> Result<usize, io::Error> {
                writer.write_all_size(&self.to_le_bytes())
            }

            #[inline]
            fn decode<R: Read>(reader: &mut R) -> Result<GluinoValue, io::Error> {
                let mut buff = [0u8; (<$type>::BITS >> 3) as usize];
                reader.read_exact(&mut buff)?;
                Ok(GluinoValue::$kind(<$type>::from_le_bytes(buff)))
            }

            #[inline]
            fn extract(value: GluinoValue) -> Result<Self, GluinoSerializationError> {
                if let GluinoValue::$kind(b) = value {
                    Ok(b)
                } else {
                    Err(GluinoSerializationError::ValueKindMismatch {
                        expected_value_kind: GluinoValueKind::$kind,
                        actual_value_kind: value.into(),
                    })
                }
            }
        }
    };
}

encode_integer_type!(u8, Uint8);
encode_integer_type!(u16, Uint16);
encode_integer_type!(u32, Uint32);
encode_integer_type!(u64, Uint64);
encode_integer_type!(u128, Uint128);
encode_integer_type!(i8, Int8);
encode_integer_type!(i16, Int16);
encode_integer_type!(i32, Int32);
encode_integer_type!(i64, Int64);
encode_integer_type!(i128, Int128);

impl Encodable for bool {
    #[inline]
    fn encode<W: Write>(&self, writer: &mut W) -> Result<usize, io::Error> {
        if *self {
            writer.write_all_size(&[1])
        } else {
            writer.write_all_size(&[0])
        }
    }

    #[inline]
    fn decode<R: Read>(reader: &mut R) -> Result<GluinoValue, io::Error> {
        let mut b = [0u8];
        reader.read_exact(&mut b)?;
        Ok(if b[0] > 0 {
            GluinoValue::Bool(true)
        } else {
            GluinoValue::Bool(false)
        })
    }

    default_extract!(Bool);
}

impl Encodable for F32 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<usize, io::Error> {
        writer.write_all_size(&self.0.to_le_bytes())
    }

    fn decode<R: Read>(reader: &mut R) -> Result<GluinoValue, io::Error> {
        let mut buff = [0u8; 4];
        reader.read_exact(&mut buff)?;
        Ok(GluinoValue::Float(F32(f32::from_le_bytes(buff))))
    }

    default_extract!(Float);
}

impl Encodable for F64 {
    fn encode<W: Write>(&self, writer: &mut W) -> Result<usize, io::Error> {
        writer.write_all_size(&self.0.to_le_bytes())
    }

    fn decode<R: Read>(reader: &mut R) -> Result<GluinoValue, io::Error> {
        let mut buff = [0u8; 8];
        reader.read_exact(&mut buff)?;
        Ok(GluinoValue::Double(F64(f64::from_le_bytes(buff))))
    }

    default_extract!(Double);
}

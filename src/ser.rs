use std::{
    collections::HashMap,
    io::{self, Write},
};

use gc::{Finalize, Gc, GcCell, Trace};
use strum::EnumDiscriminants;
use strum_macros::EnumIter;

use crate::{
    compiled_spec::{CompiledSpec, CompiledSpecStructure},
    spec::{
        combine, InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size,
        StringEncodingFmt,
    },
    util::{variable_length_encode_u64, WriteAllReturnSize},
};
pub trait GluinoSpecType {
    fn get_spec() -> CompiledSpec;
}

#[derive(Debug, PartialEq, Clone, EnumDiscriminants)]
#[strum_discriminants(name(GluinoValueKind))]
#[strum_discriminants(derive(EnumIter))]
pub enum GluinoValue {
    /// native to rust
    /// simple types
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(i128),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Uint128(u128),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Float(f32),
    Double(f64),
    /// Compound
    Optional(Option<Box<GluinoValue>>),
    List(Vec<GluinoValue>),
    Map(Vec<(GluinoValue, GluinoValue)>),
    Record(Vec<GluinoValue>),
    Tuple(Vec<GluinoValue>),
    Enum(u64, Box<GluinoValue>),
    Union(u64, Box<GluinoValue>),
    //non native
    BigInt(u8, Vec<u8>),
    BigUint(u8, Vec<u8>),
    BinaryFloatingPoint(InterchangeBinaryFloatingPointFormat, Vec<u8>),
    DecimalFloatingPoint(InterchangeDecimalFloatingPointFormat, Vec<u8>),
    NonUtf8String(StringEncodingFmt, Vec<u8>),
    Decimal(Vec<u8>),
    //void
    Void,
}

pub trait GluinoValueSer<W>
where
    W: Write,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError>;
}

trait SerSizeValidator {
    fn valiate_size(&self, size: u64) -> bool;
    fn need_write_size(&self) -> bool;
}

impl SerSizeValidator for Size {
    fn valiate_size(&self, data_size: u64) -> bool {
        match self {
            Self::Variable => true,
            Self::Fixed(n) => n == &data_size,
            Self::Range(r) => r.start <= data_size && data_size <= r.end,
        }
    }

    fn need_write_size(&self) -> bool {
        match self {
            Self::Variable | Self::Range(_) => true,
            Self::Fixed(_) => false,
        }
    }
}

#[derive(Trace, Finalize)]
struct VoidGluinoValueSer;

impl<W> GluinoValueSer<W> for VoidGluinoValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(&self, value: GluinoValue, _: &mut W) -> Result<usize, GluinoSerializationError> {
        if matches!(value, GluinoValue::Void) {
            Ok(0)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValue::Void.into(),
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct BoolGluinoValueSer;

impl<W> GluinoValueSer<W> for BoolGluinoValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Bool(b) = value {
            Ok(if b {
                writer.write_all_size(&[1])?
            } else {
                writer.write_all_size(&[0])?
            })
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Bool,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct I8ValueSer;

impl<W> GluinoValueSer<W> for I8ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Int8(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Int8,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct I16ValueSer;

impl<W> GluinoValueSer<W> for I16ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Int16(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Int16,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct I32ValueSer;

impl<W> GluinoValueSer<W> for I32ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Int32(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Int32,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct I64ValueSer;

impl<W> GluinoValueSer<W> for I64ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Int64(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Int64,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct I128ValueSer;

impl<W> GluinoValueSer<W> for I128ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Int128(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Int128,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct BigIntValueSer {
    n: u8,
}

impl<W> GluinoValueSer<W> for BigIntValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::BigInt(v, bytes) = value {
            if self.n == v && (bytes.len() >> v) == 1 {
                Ok(writer.write_all_size(&bytes[..])?)
            } else {
                //wrong format
                todo!()
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::BigInt,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct U8ValueSer;

impl<W> GluinoValueSer<W> for U8ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Uint8(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Uint8,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct U16ValueSer;

impl<W> GluinoValueSer<W> for U16ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Uint16(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Uint16,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct U32ValueSer;

impl<W> GluinoValueSer<W> for U32ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Uint32(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Uint32,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct U64ValueSer;

impl<W> GluinoValueSer<W> for U64ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Uint64(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Uint64,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct U128ValueSer;

impl<W> GluinoValueSer<W> for U128ValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Uint128(v) = value {
            Ok(writer.write_all_size(&v.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Uint128,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct BigUintValueSer {
    n: u8,
}

impl<W> GluinoValueSer<W> for BigUintValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::BigUint(v, bytes) = value {
            if self.n == v && (bytes.len() >> v) == 1 {
                Ok(writer.write_all_size(&bytes[..])?)
            } else {
                //wrong format
                todo!()
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::BigUint,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct FloatValueSer;

impl<W> GluinoValueSer<W> for FloatValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Float(f) = value {
            Ok(writer.write_all_size(&f.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Float,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct DoubleValueSer;

impl<W> GluinoValueSer<W> for DoubleValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Double(f) = value {
            Ok(writer.write_all_size(&f.to_le_bytes())?)
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Double,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct BinaryFloatingPointValueSer {
    fmt: InterchangeBinaryFloatingPointFormat,
}

impl<W> GluinoValueSer<W> for BinaryFloatingPointValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::BinaryFloatingPoint(target_fmt, bytes) = value {
            if self.fmt == target_fmt
                && (target_fmt.significand_bits() + target_fmt.exponent_bits()) >> 3
                    == bytes.len() as u64
            {
                Ok(writer.write_all_size(&bytes)?)
            } else {
                //format error
                todo!()
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::BinaryFloatingPoint,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct DecimalFloatingPointValueSer {
    fmt: InterchangeDecimalFloatingPointFormat,
}

impl<W> GluinoValueSer<W> for DecimalFloatingPointValueSer
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::DecimalFloatingPoint(target_fmt, bytes) = value {
            if self.fmt == target_fmt && target_fmt.minimum_byes_needed() == bytes.len() {
                Ok(writer.write_all_size(&bytes)?)
            } else {
                //format error
                todo!()
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::DecimalFloatingPoint,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct DecimalSer;

impl<W> GluinoValueSer<W> for DecimalSer
where
    for<'a> W: Write + 'a,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Decimal(bytes) = value {
            let v_size = variable_length_encode_u64(bytes.len() as u64, writer)?;
            writer.write_all_size(&bytes)?;
            Ok(v_size + bytes.len())
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Decimal,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct ByteValueSer {
    #[unsafe_ignore_trace]
    spec_size: Size,
}

impl<W> GluinoValueSer<W> for ByteValueSer
where
    for<'a> W: Write + 'a,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Bytes(bytes) = value {
            let size = bytes.len() as u64;
            if self.spec_size.valiate_size(size) {
                let v_size = if self.spec_size.need_write_size() {
                    variable_length_encode_u64(size, writer)?
                } else {
                    0
                };
                writer.write_all_size(&bytes[0..bytes.len()])?;
                Ok(v_size + bytes.len())
            } else {
                Err(GluinoSerializationError::IncorrectDataSize {
                    expected_size: self.spec_size.clone(),
                    actual_size: size,
                    size_value_kind: GluinoValueKind::Bytes,
                })
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Bytes,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct Utf8Ser {
    #[unsafe_ignore_trace]
    spec_size: Size,
}

impl<W> GluinoValueSer<W> for Utf8Ser
where
    for<'a> W: Write + 'a,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::String(s) = value {
            let bytes = s.as_bytes();
            let size = bytes.len() as u64;
            if self.spec_size.valiate_size(size) {
                let v_size = if self.spec_size.need_write_size() {
                    variable_length_encode_u64(size, writer)?
                } else {
                    0
                };
                writer.write_all_size(bytes)?;
                Ok(v_size + bytes.len())
            } else {
                Err(GluinoSerializationError::IncorrectDataSize {
                    expected_size: self.spec_size.clone(),
                    actual_size: size,
                    size_value_kind: GluinoValueKind::String,
                })
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::String,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct NonUtf8Ser {
    #[unsafe_ignore_trace]
    spec_size: Size,
    #[unsafe_ignore_trace]
    fmt: StringEncodingFmt,
}

impl<W> GluinoValueSer<W> for NonUtf8Ser
where
    for<'a> W: Write + 'a,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::NonUtf8String(fmt, bytes) = value {
            if fmt == self.fmt {
                let size = bytes.len() as u64;
                if self.spec_size.valiate_size(size) {
                    let v_size = if self.spec_size.need_write_size() {
                        variable_length_encode_u64(size, writer)?
                    } else {
                        0
                    };
                    writer.write_all_size(&bytes)?;
                    Ok(v_size + bytes.len())
                } else {
                    Err(GluinoSerializationError::IncorrectDataSize {
                        expected_size: self.spec_size.clone(),
                        actual_size: size,
                        size_value_kind: GluinoValueKind::NonUtf8String,
                    })
                }
            } else {
                //wrong fmt
                todo!()
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::NonUtf8String,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct MapSer<W> {
    #[unsafe_ignore_trace]
    spec_size: Size,
    key_ser: Box<dyn GluinoValueSer<W>>,
    value_ser: Box<dyn GluinoValueSer<W>>,
}

impl<W> GluinoValueSer<W> for MapSer<W>
where
    W: Write,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Map(values) = value {
            let size = values.len() as u64;
            if self.spec_size.valiate_size(size) {
                let written = if self.spec_size.need_write_size() {
                    Ok(variable_length_encode_u64(size, writer)?)
                } else {
                    Ok(0)
                };
                values
                    .into_iter()
                    .map(|(key, value)| {
                        combine(
                            self.key_ser.serialize(key, writer),
                            self.value_ser.serialize(value, writer),
                        )
                    })
                    .fold(written, combine)
            } else {
                Err(GluinoSerializationError::IncorrectDataSize {
                    expected_size: self.spec_size.clone(),
                    actual_size: size,
                    size_value_kind: GluinoValueKind::Map,
                })
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Map,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct ListSer<W> {
    #[unsafe_ignore_trace]
    spec_size: Size,
    value_ser: Box<dyn GluinoValueSer<W>>,
}

impl<W> GluinoValueSer<W> for ListSer<W>
where
    for<'a> W: Write + 'a,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::List(values) = value {
            let size = values.len() as u64;
            if self.spec_size.valiate_size(size) {
                let written = if self.spec_size.need_write_size() {
                    Ok(variable_length_encode_u64(size, writer)?)
                } else {
                    Ok(0)
                };
                values
                    .into_iter()
                    .map(|value: GluinoValue| self.value_ser.serialize(value, writer))
                    .fold(written, combine)
            } else {
                Err(GluinoSerializationError::IncorrectDataSize {
                    expected_size: self.spec_size.clone(),
                    actual_size: size,
                    size_value_kind: GluinoValueKind::List,
                })
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::List,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct OptionalValueSer<W> {
    inner_ser: Box<dyn GluinoValueSer<W>>,
}

impl<W> GluinoValueSer<W> for OptionalValueSer<W>
where
    for<'a> W: Write + 'a,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Optional(optional_value) = value {
            match optional_value {
                Some(value) => {
                    writer.write_all_size(&[1])?;
                    Ok(1 + self.inner_ser.serialize(*value, writer)?)
                }
                None => {
                    writer.write_all_size(&[0])?;
                    Ok(1)
                }
            }
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Optional,
                actual_value_kind: value.into(),
            })
        }
    }
}

#[derive(Trace, Finalize)]
struct ProductValueSer<W> {
    field_sers: Vec<Box<dyn GluinoValueSer<W>>>,
}

impl<W> GluinoValueSer<W> for ProductValueSer<W>
where
    for<'a> W: Write + 'a,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Record(fields) | GluinoValue::Tuple(fields) = value {
            if fields.len() == self.field_sers.len() {
                fields
                    .into_iter()
                    .zip(self.field_sers.iter())
                    .map(|(field, ser)| ser.serialize(field, writer))
                    .fold(Ok(0), combine)
            } else {
                Err(GluinoSerializationError::IncorrectNumberOfFields {
                    correct_number_of_fields: self.field_sers.len(),
                    actual_number_of_fields: fields.len(),
                })
            }
        } else {
            Err(GluinoSerializationError::ProductKindValueKindMismatch {
                actual_value_kind: value.into(),
            })
        }
    }
}

struct SumValueSer<W> {
    varient_sers: HashMap<u64, Box<dyn GluinoValueSer<W>>>,
}

impl<W> GluinoValueSer<W> for SumValueSer<W>
where
    for<'a> W: Write + 'a,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Enum(variant_id, value) | GluinoValue::Union(variant_id, value) = value
        {
            if let Some(variant_ser) = self.varient_sers.get(&variant_id) {
                Ok(variable_length_encode_u64(variant_id, writer)?
                    + variant_ser.serialize(*value, writer)?)
            } else {
                Err(GluinoSerializationError::InvalidVariantId {
                    variant_id: variant_id.clone() as usize,
                    max_variant_id: self.varient_sers.len() - 1,
                })
            }
        } else {
            Err(GluinoSerializationError::SumKindValueKindMismatch {
                actual_value_kind: value.into(),
            })
        }
    }
}

impl<W> GluinoValueSer<W> for Gc<GcCell<Box<dyn GluinoValueSer<W>>>>
where
    for<'a> W: Write + 'a,
    for<'x> (dyn GluinoValueSer<W>): Trace + Finalize + 'x,
{
    #[inline]
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        self.borrow().serialize(value, writer)
    }
}

pub fn get_unit_serialization_function<W>(spec: &CompiledSpec) -> Box<dyn GluinoValueSer<W>>
where
    for<'ser> (dyn GluinoValueSer<W>): Trace + Finalize + 'ser,
    for<'write> W: Write + 'write,
{
    get_unit_serialization_function_internal::<W>(spec, &mut HashMap::new())
}

fn get_unit_serialization_function_internal<W>(
    spec: &CompiledSpec,
    named_unit_sers: &mut HashMap<String, Gc<GcCell<Box<dyn GluinoValueSer<W>>>>>,
) -> Box<dyn GluinoValueSer<W>>
where
    for<'ser> (dyn GluinoValueSer<W>): Trace + Finalize + 'ser,
    for<'write> W: Write + 'write,
{
    match spec.structure() {
        CompiledSpecStructure::Void => Box::new(VoidGluinoValueSer),
        CompiledSpecStructure::Bool => Box::new(BoolGluinoValueSer),
        CompiledSpecStructure::Uint(n) => match n {
            0 => Box::new(U8ValueSer),
            1 => Box::new(U16ValueSer),
            2 => Box::new(U32ValueSer),
            3 => Box::new(U64ValueSer),
            4 => Box::new(U128ValueSer),
            _ => {
                let n = n.clone();
                Box::new(BigUintValueSer { n })
            }
        },
        CompiledSpecStructure::Int(n) => match n {
            0 => Box::new(I8ValueSer),
            1 => Box::new(I16ValueSer),
            2 => Box::new(I32ValueSer),
            3 => Box::new(I64ValueSer),
            4 => Box::new(I128ValueSer),
            _ => {
                let n = n.clone();
                Box::new(BigIntValueSer { n })
            }
        },
        CompiledSpecStructure::BinaryFloatingPoint(fmt) => match fmt {
            InterchangeBinaryFloatingPointFormat::Single => Box::new(FloatValueSer),
            InterchangeBinaryFloatingPointFormat::Double => Box::new(DoubleValueSer),
            _ => {
                let fmt = fmt.clone();
                Box::new(BinaryFloatingPointValueSer { fmt })
            }
        },
        CompiledSpecStructure::DecimalFloatingPoint(fmt) => {
            let fmt = fmt.clone();
            Box::new(DecimalFloatingPointValueSer { fmt })
        }
        CompiledSpecStructure::Decimal(_) => {
            //standardize on serialization of decimal type
            Box::new(DecimalSer)
        }
        CompiledSpecStructure::Bytes(size) => Box::new(ByteValueSer {
            spec_size: size.clone(),
        }),
        CompiledSpecStructure::String(size, fmt) => match fmt {
            StringEncodingFmt::Utf8 => Box::new(Utf8Ser {
                spec_size: size.clone(),
            }),
            StringEncodingFmt::Utf16 | StringEncodingFmt::Ascii => Box::new(NonUtf8Ser {
                fmt: fmt.clone(),
                spec_size: size.clone(),
            }),
        },
        CompiledSpecStructure::Map {
            size,
            key_spec,
            value_spec,
        } => {
            let key_ser = get_unit_serialization_function_internal::<W>(key_spec, named_unit_sers);
            let value_ser =
                get_unit_serialization_function_internal::<W>(value_spec, named_unit_sers);
            Box::new(MapSer {
                spec_size: size.clone(),
                key_ser,
                value_ser,
            })
        }
        CompiledSpecStructure::List { size, value_spec } => {
            let value_ser =
                get_unit_serialization_function_internal::<W>(value_spec, named_unit_sers);
            Box::new(ListSer {
                spec_size: size.clone(),
                value_ser,
            })
        }
        CompiledSpecStructure::Optional(inner) => {
            let inner_ser = get_unit_serialization_function_internal::<W>(inner, named_unit_sers);
            Box::new(OptionalValueSer { inner_ser })
        }
        CompiledSpecStructure::Record {
            fields,
            field_to_spec,
            ..
        } => Box::new(ProductValueSer {
            field_sers: fields
                .iter()
                .map(|field| field_to_spec.get(field).unwrap())
                .map(|spec| get_unit_serialization_function_internal::<W>(spec, named_unit_sers))
                .collect(),
        }),
        CompiledSpecStructure::Tuple(fields) => Box::new(ProductValueSer {
            field_sers: fields
                .iter()
                .map(|spec| get_unit_serialization_function_internal::<W>(spec, named_unit_sers))
                .collect(),
        }),
        CompiledSpecStructure::Enum {
            variants,
            variant_to_spec,
        } => Box::new(SumValueSer {
            varient_sers: variants
                .iter()
                .map(|variant| variant_to_spec.get(variant).unwrap())
                .map(|spec| get_unit_serialization_function_internal::<W>(spec, named_unit_sers))
                .enumerate()
                .map(|(a, b)| (a as u64, b))
                .collect(),
        }),
        CompiledSpecStructure::Union(variants) => Box::new(SumValueSer {
            varient_sers: variants
                .iter()
                .map(|spec| get_unit_serialization_function_internal::<W>(spec, named_unit_sers))
                .enumerate()
                .map(|(a, b)| (a as u64, b))
                .collect(),
        }),
        CompiledSpecStructure::Name(name) => match named_unit_sers.get(name) {
            Some(ser) => Box::new(ser.clone()),
            None => {
                let named_ser: Gc<GcCell<Box<dyn GluinoValueSer<W>>>> =
                    Gc::new(GcCell::new(Box::new(VoidGluinoValueSer)));
                named_unit_sers.insert(name.clone(), named_ser.clone());
                let inner_ser = spec
                    .named_schema()
                    .get(name)
                    .expect("Compiled spec should have named spec")
                    .use_ref(|spec| {
                        get_unit_serialization_function_internal::<W>(spec, named_unit_sers)
                    });
                *named_ser.borrow_mut() = inner_ser;
                Box::new(named_ser)
            }
        },
    }
}

pub enum GluinoSerializationError {
    WriteError(io::Error),
    IncorrectDataSize {
        expected_size: Size,
        actual_size: u64,
        size_value_kind: GluinoValueKind,
    },
    InvalidVariantId {
        variant_id: usize,
        max_variant_id: usize,
    },
    ValueKindMismatch {
        expected_value_kind: GluinoValueKind,
        actual_value_kind: GluinoValueKind,
    },
    ProductKindValueKindMismatch {
        actual_value_kind: GluinoValueKind,
    },
    IncorrectNumberOfFields {
        correct_number_of_fields: usize,
        actual_number_of_fields: usize,
    },
    SumKindValueKindMismatch {
        actual_value_kind: GluinoValueKind,
    },
}

impl From<io::Error> for GluinoSerializationError {
    fn from(e: io::Error) -> Self {
        GluinoSerializationError::WriteError(e)
    }
}

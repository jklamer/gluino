use std::{collections::HashMap, io::Write, marker::PhantomData};

use gc::{Finalize, Trace};

use crate::{
    spec::{
        combine, InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size,
    },
    util::{variable_length_encode_u64, WriteAllReturnSize},
};

use super::{
    encode::Encodable, GluinoSerializationError, GluinoValue, GluinoValueKind, GluinoValueSer,
};

trait SerSizeValidator {
    fn validate_size(&self, size: u64) -> bool;
    fn need_write_size(&self) -> bool;
}

impl SerSizeValidator for Size {
    #[inline]
    fn validate_size(&self, data_size: u64) -> bool {
        match self {
            Self::Variable => true,
            Self::Fixed(n) => n == &data_size,
            Self::Range(r) => r.start <= data_size && data_size < r.end,
            Self::GreaterThan(n) => n <= &data_size,
            Self::LessThan(n) => n > &data_size,
        }
    }

    fn need_write_size(&self) -> bool {
        match self {
            Self::Variable | Self::Range(_) | Self::LessThan(_) | Self::GreaterThan(_) => true,
            Self::Fixed(_) => false,
        }
    }
}

#[derive(Trace, Finalize)]
pub(crate) struct VoidGluinoValueSer;

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
pub(crate) struct BoolGluinoValueSer;

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
pub(crate) struct NativeSingleSer<E: Encodable> {
    _d: PhantomData<E>,
}

impl<E: Encodable> NativeSingleSer<E> {
    pub(crate) fn new() -> Self {
        NativeSingleSer::<E> { _d: PhantomData }
    }
}

impl<W, E> GluinoValueSer<W> for NativeSingleSer<E>
where
    E: Encodable,
    W: Write,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        Ok(E::extract(value)?.encode(writer)?)
    }
}

#[derive(Trace, Finalize)]
pub(crate) struct BigIntValueSer {
    pub(crate) n: u8,
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
            if bytes.len() >> self.n == 1 {
                Ok(writer.write_all_size(&bytes[..])?)
            } else {
                Err(GluinoSerializationError::InncorrectNumberOfIntegerBytes {
                    expect_bytes: 1 << self.n,
                    actual_bytes: bytes.len(),
                })
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
pub(crate) struct BigUintValueSer {
    pub(crate) n: u8,
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
            if bytes.len() >> v == 1 {
                Ok(writer.write_all_size(&bytes[..])?)
            } else {
                // wrong format
                Err(GluinoSerializationError::InncorrectNumberOfIntegerBytes {
                    expect_bytes: 1 << self.n,
                    actual_bytes: bytes.len(),
                })
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
pub(crate) struct BinaryFloatingPointValueSer {
    pub(crate) fmt: InterchangeBinaryFloatingPointFormat,
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
                Err(
                    GluinoSerializationError::IncorrectNumberOfFloatingPointBytes {
                        expext_bytes: (target_fmt.significand_bits() + target_fmt.exponent_bits())
                            as usize
                            >> 3,
                        actual_bytes: bytes.len(),
                    },
                )
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
pub(crate) struct DecimalFloatingPointValueSer {
    pub(crate) fmt: InterchangeDecimalFloatingPointFormat,
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
                Err(
                    GluinoSerializationError::IncorrectNumberOfFloatingPointBytes {
                        expext_bytes: target_fmt.minimum_byes_needed(),
                        actual_bytes: bytes.len(),
                    },
                )
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
pub(crate) struct DecimalSer;

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
pub(crate) struct ByteValueSer {
    #[unsafe_ignore_trace]
    pub(crate) spec_size: Size,
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
            if self.spec_size.validate_size(size) {
                let v_size = if self.spec_size.need_write_size() {
                    variable_length_encode_u64(size, writer)?
                } else {
                    0
                };
                Ok(v_size +  writer.write_all_size(&bytes[0..bytes.len()])?)
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
pub(crate) struct Utf8Ser {
    #[unsafe_ignore_trace]
    pub(crate) spec_size: Size,
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
            if self.spec_size.validate_size(size) {
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
pub(crate) struct NonUtf8Ser {
    #[unsafe_ignore_trace]
    pub(crate) spec_size: Size,
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
        if let GluinoValue::NonUtf8String(bytes) = value {
            let size = bytes.len() as u64;
            if self.spec_size.validate_size(size) {
                let v_size = if self.spec_size.need_write_size() {
                    variable_length_encode_u64(size, writer)?
                } else {
                    0
                };
                Ok(v_size + writer.write_all_size(&bytes)?)
            } else {
                Err(GluinoSerializationError::IncorrectDataSize {
                    expected_size: self.spec_size.clone(),
                    actual_size: size,
                    size_value_kind: GluinoValueKind::NonUtf8String,
                })
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
pub(crate) struct MapSer<W> {
    #[unsafe_ignore_trace]
    pub(crate) spec_size: Size,
    pub(crate) key_ser: Box<dyn GluinoValueSer<W>>,
    pub(crate) value_ser: Box<dyn GluinoValueSer<W>>,
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
            if self.spec_size.validate_size(size) {
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
pub(crate) struct ListSer<W> {
    #[unsafe_ignore_trace]
    pub(crate) spec_size: Size,
    pub(crate) value_ser: Box<dyn GluinoValueSer<W>>,
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
            if self.spec_size.validate_size(size) {
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
pub(crate) struct OptionalValueSer<W> {
    pub(crate) inner_ser: Box<dyn GluinoValueSer<W>>,
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
pub(crate) struct ProductValueSer<W> {
    pub(crate) field_sers: Vec<Box<dyn GluinoValueSer<W>>>,
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

pub(crate) struct SumValueSer<W> {
    pub(crate) varient_sers: HashMap<u64, Box<dyn GluinoValueSer<W>>>,
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

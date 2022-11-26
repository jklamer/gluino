use std::{io::{self, Write}, collections::HashMap};

use strum::EnumDiscriminants;
use strum_macros::EnumIter;

use crate::{
    compiled_spec::{CompiledSpec, CompiledSpecStructure},
    spec::{Size, combine, InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat},
    util::variable_length_encode_u64,
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
                writer.write(&[1])?
            } else {
                writer.write(&[0])?
            })
        } else {
            Err(GluinoSerializationError::ValueKindMismatch {
                expected_value_kind: GluinoValueKind::Bool,
                actual_value_kind: value.into(),
            })
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            // MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            // MISMATCH ERROR
            todo!()
        }
    }
}

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
                Ok(writer.write(&bytes[..])?)
            } else {
                //wrong format
                todo!()
            }
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            // MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            // MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            // MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            // MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&v.to_le_bytes())?)
        } else {
            // MISMATCH ERROR
            todo!()
        }
    }
}

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
                Ok(writer.write(&bytes[..])?)
            } else {
                //wrong format
                todo!()
            }
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&f.to_le_bytes())?)
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
            Ok(writer.write(&f.to_le_bytes())?)
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
                Ok(writer.write(&bytes)?)
            } else {
                //format error
                todo!()
            }
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

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
                Ok(writer.write(&bytes)?)
            } else {
                //format error
                todo!()
            }
        } else {
            //MISMATCH ERROR
            todo!()
        }
    }
}

struct FixedByteValueSer {
    n: u64
}

impl <W> GluinoValueSer<W> for FixedByteValueSer 
where for<'a> W: Write + 'a{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Bytes(bytes) = value {
            if bytes.len() as u64 == self.n {
                writer.write_all(&bytes[0..bytes.len()])?;
                Ok(bytes.len())
            } else {
                // wrong size
                todo!()
            }
        } else {
            //mismatched type
            todo!()
        }
    }
}

struct VariableByteValueSer;

impl <W> GluinoValueSer<W> for VariableByteValueSer 
where for<'a> W: Write + 'a{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Bytes(bytes) = value {
            let v_size = variable_length_encode_u64(bytes.len() as u64, writer)?;
            writer.write_all(&bytes)?;
            Ok(v_size + bytes.len())
        } else {
            //mismatched type
            todo!()
        }
    }
}

struct FixedSizeMapSer<W> {
    n: u64,
    key_ser: Box<dyn GluinoValueSer<W>>,
    value_ser: Box<dyn GluinoValueSer<W>>,
}

impl<W> GluinoValueSer<W> for FixedSizeMapSer<W>
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
            if values.len() as u64 == self.n {
                values
                    .into_iter()
                    .map(|(key, value)| {
                        combine(
                            self.key_ser.serialize(key, writer),
                            self.value_ser.serialize(value, writer),
                        )
                    })
                    .fold(Ok(0), combine)
            } else {
                //wrong size!
                todo!()
            }
        } else {
            //mismatch error
            todo!()
        }
    }
}

struct VariableSizeMapSer<W> {
    key_ser: Box<dyn GluinoValueSer<W>>,
    value_ser: Box<dyn GluinoValueSer<W>>,
}

impl<W> GluinoValueSer<W> for VariableSizeMapSer<W>
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
            Ok(variable_length_encode_u64(values.len() as u64, writer)?
                + values
                    .into_iter()
                    .map(|(key, value)| {
                        combine(
                            self.key_ser.serialize(key, writer),
                            self.value_ser.serialize(value, writer),
                        )
                    })
                    .fold(Ok(0), combine)?)
        } else {
            //mismatch error
            todo!()
        }
    }
}

struct FixedSizeListSer<W> {
    n: u64,
    value_ser: Box<dyn GluinoValueSer<W>>
}

impl <W> GluinoValueSer<W> for FixedSizeListSer<W> 
where
for<'a> W: Write + 'a
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::List(values) = value {
            if values.len() as u64 == self.n {
                values
                    .into_iter()
                    .map(|value: GluinoValue| {
                        self.value_ser.serialize(value, writer)
                    })
                    .fold(Ok(0), combine)
            } else {
                //wrong size!
                todo!()
            }
        } else {
            //mismatch error
            todo!()
        }
    }
}

struct VariableSizeListSer<W> {
    value_ser: Box<dyn GluinoValueSer<W>>
}

impl <W> GluinoValueSer<W> for VariableSizeListSer<W> 
where
for<'a> W: Write + 'a
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::List(values) = value {
            Ok(variable_length_encode_u64(values.len() as u64, writer)? +
                values
                    .into_iter()
                    .map(|value: GluinoValue| {
                        self.value_ser.serialize(value, writer)
                    })
                    .fold(Ok(0), combine)?)
        } else {
            //mismatch error
            todo!()
        }
    }
}

struct OptionalValueSer<W> {
    inner_ser: Box<dyn GluinoValueSer<W>>
}

impl <W> GluinoValueSer<W> for OptionalValueSer<W> 
where 
for<'a> W: Write + 'a{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Optional(optional_value) = value {
            match optional_value {
                Some(value) => {
                    writer.write_all(&[1])?;
                    Ok(1 + self.inner_ser.serialize(*value, writer)?)
                },
                None => {writer.write_all(&[0])?; Ok(1)}
            }
        } else {
            //TODO mismatch error
            todo!()
        }
    }
}

struct ProductValueSer<W> {
    field_sers: Vec<Box<dyn GluinoValueSer<W>>>
}

impl <W> GluinoValueSer<W> for ProductValueSer<W> 
where for<'a> W: Write + 'a {
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Record(fields) | GluinoValue::Tuple(fields) = value {
            if fields.len() == self.field_sers.len() {
                fields.into_iter().zip(self.field_sers.iter())
                .map(|(field, ser)| {
                    ser.serialize(field, writer)
                })
                .fold(Ok(0), combine)
            } else {
                //wrong size
                todo!()
            }
        }else {
            //mismatched
            todo!()
        }
    }
}

struct SumValueSer<W> {
    varient_sers: HashMap<u64, Box<dyn GluinoValueSer<W>>>
}

impl <W> GluinoValueSer<W> for SumValueSer<W> 
where for<'a> W: Write + 'a
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        if let GluinoValue::Enum(variant_id, value) | GluinoValue::Union(variant_id, value) = value {
            if let Some(variant_ser) = self.varient_sers.get(&variant_id) {
                Ok(
                    variable_length_encode_u64(variant_id, writer)? +
                    variant_ser.serialize(*value, writer)?
                )
            } else {
                //invalid variant id
                todo!()
            }
        } else {
            //mismatch
            todo!()
        }
    }
}

pub fn get_unit_serialization_function<W>(spec: &CompiledSpec) -> Box<dyn GluinoValueSer<W>>
where
    for <'a> W: Write + 'a,
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
        CompiledSpecStructure::Decimal(fmt) => {
            //standardize on serialization of decimal type
            todo!();
        },
        CompiledSpecStructure::Bytes(size) => {
            match size {
                Size::Fixed(n) => Box::new(FixedByteValueSer{n:n.clone()}),
                Size::Variable => Box::new(VariableByteValueSer)
            }
        },
        CompiledSpecStructure::String(size, fmt) => todo!(), 
        CompiledSpecStructure::Map {
            size,
            key_spec,
            value_spec,
        } => {
            let key_ser = get_unit_serialization_function::<W>(key_spec);
            let value_ser = get_unit_serialization_function::<W>(value_spec);
            match size {
                Size::Fixed(n) => Box::new(FixedSizeMapSer{ n:n.clone(), key_ser, value_ser}),
                Size::Variable => Box::new(VariableSizeMapSer{key_ser, value_ser})
            }
        },
        CompiledSpecStructure::List { size, value_spec } => {
            let value_ser = get_unit_serialization_function::<W>(value_spec);
            match size {
                Size::Fixed(n) => Box::new(FixedSizeListSer{n: n.clone(), value_ser}),
                Size::Variable => Box::new(VariableSizeListSer{value_ser}),
            }
        },
        CompiledSpecStructure::Optional(inner) => {
            let inner_ser = get_unit_serialization_function::<W>(inner);
            Box::new(OptionalValueSer{inner_ser})
        },
        CompiledSpecStructure::Record {
            fields,
            field_to_spec,
            ..
        } => {
            Box::new(ProductValueSer {
                field_sers: fields.iter().map(|field| field_to_spec.get(field).unwrap())
                .map(|spec| get_unit_serialization_function::<W>(spec))
                .collect()
            })
        },
        CompiledSpecStructure::Tuple(fields) => Box::new(ProductValueSer {
            field_sers: fields.iter().map(|spec| get_unit_serialization_function::<W>(spec)).collect()
        }),
        CompiledSpecStructure::Enum {
            variants,
            variant_to_spec,
        } => {
            Box::new(SumValueSer{
                varient_sers: variants.iter()
                .map(|variant| variant_to_spec.get(variant).unwrap())
                .map(|spec| get_unit_serialization_function::<W>(spec))
                .enumerate()
                .map(|(a,b)|(a as u64, b))
                .collect()
            })  
        },
        CompiledSpecStructure::Union(variants) => {
            Box::new(SumValueSer{
                varient_sers: variants.iter()
                .map(|spec| get_unit_serialization_function::<W>(spec))
                .enumerate()
                .map(|(a,b)|(a as u64, b))
                .collect()
            })
        },
        CompiledSpecStructure::Name(name) => todo!(),
    }
}

pub enum GluinoSerializationError {
    WriteError(io::Error),
    ValueKindMismatch {
        expected_value_kind: GluinoValueKind,
        actual_value_kind: GluinoValueKind,
    },
}

impl From<io::Error> for GluinoSerializationError {
    fn from(e: io::Error) -> Self {
        GluinoSerializationError::WriteError(e)
    }
}

use std::{
    borrow::Borrow,
    collections::HashMap,
    io::{self, Write},
    rc::{Rc, Weak},
    sync::Arc,
};

use strum::EnumDiscriminants;
use strum_macros::EnumIter;

use crate::spec::Size::Fixed;
use crate::{
    compiled_spec::{CompiledSpec, CompiledSpecStructure},
    spec::{InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat},
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
    fn serialize(
        &self,
        value: GluinoValue,
        _: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
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

pub fn get_unit_serialization_function<W>(spec: &CompiledSpec) -> Box<dyn GluinoValueSer<W>>
where
    W: Write,
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
        }
        CompiledSpecStructure::Map {
            size,
            key_spec,
            value_spec,
        } => {
            todo!()
        }
        CompiledSpecStructure::List { size, value_spec } => todo!(),
        CompiledSpecStructure::String(size, fmt) => todo!(),
        CompiledSpecStructure::Bytes(size) => todo!(),
        CompiledSpecStructure::Optional(inner) => todo!(),
        CompiledSpecStructure::Name(name) => todo!(),
        CompiledSpecStructure::Record {
            fields,
            field_to_spec,
            field_to_index,
        } => todo!(),
        CompiledSpecStructure::Tuple(fields) => todo!(),
        CompiledSpecStructure::Enum {
            variants,
            variant_to_spec,
        } => todo!(),
        CompiledSpecStructure::Union(variants) => todo!(),
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

// impl<T: GluinoSpecType> serde::Serializer for GluinoSerializer<T> {
//     type Ok = GluinoValue;

//     type Error = SerializationError;

//     type SerializeSeq;

//     type SerializeTuple;

//     type SerializeTupleStruct;

//     type SerializeTupleVariant;

//     type SerializeMap;

//     type SerializeStruct;

//     type SerializeStructVariant;

//     fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_some<S: ?Sized>(self, value: &S) -> Result<Self::Ok, Self::Error>
//     where
//         S: serde::Serialize,
//     {
//         todo!()
//     }

//     fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_unit_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//     ) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }

//     fn serialize_newtype_struct<S: ?Sized>(
//         self,
//         name: &'static str,
//         value: &S,
//     ) -> Result<Self::Ok, Self::Error>
//     where
//         S: serde::Serialize,
//     {
//         todo!()
//     }

//     fn serialize_newtype_variant<S: ?Sized>(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         value: &S,
//     ) -> Result<Self::Ok, Self::Error>
//     where
//         S: serde::Serialize,
//     {
//         todo!()
//     }

//     fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleStruct, Self::Error> {
//         todo!()
//     }

//     fn serialize_tuple_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleVariant, Self::Error> {
//         todo!()
//     }

//     fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
//         todo!()
//     }

//     fn serialize_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStruct, Self::Error> {
//         todo!()
//     }

//     fn serialize_struct_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStructVariant, Self::Error> {
//         todo!()
//     }
// }

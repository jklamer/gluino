mod de_impls;
mod ser_impls;
#[macro_use]
mod encode;

use std::{
    collections::HashMap,
    io::{self, Read, Write},
};
use std::cell::RefCell;
use std::rc::Rc;
use strum::{EnumDiscriminants, EnumIter};

use crate::{
    compiled_spec::{CompiledSpec, CompiledSpecStructure},
    spec::{
        InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size,
        StringEncodingFmt,
    },
};
use crate::serde::encode::Encodable;
use crate::util::WriteAllReturnSize;
use self::{ser_impls::*, de_impls::{NativeSingleDe, VoidGluinoValueDe}};

pub trait GluinoSpecType {
    fn get_spec() -> CompiledSpec;
}

#[derive(Eq, Debug, PartialEq, Clone, EnumDiscriminants)]
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
    Float(F32),
    Double(F64),
    String(String),
    Bytes(Vec<u8>),
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
    NonUtf8String(Vec<u8>),
    Decimal(Vec<u8>),
    ConstSet(u64),
    //void
    Void,
}

// Custom struct to wrap f32 for Eq and Ord
#[derive(Debug, Clone, Copy)]
pub struct F32(f32);

impl AsRef<f32> for F32 {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl PartialEq for F32 {
    fn eq(&self, other: &Self) -> bool {
        // For Eq, we want NaN == NaN.
        // For all other values, compare them normally.
        if self.0.is_nan() {
            other.0.is_nan() // Treat NaNs as equal for the purpose of Eq
        } else {
            self.0 == other.0 // Standard comparison for non-NaNs
        }
    }
}

// f32 is not Ord because NaN cannot be ordered.
// Implementing Ord also requires that NaN is consistent with the ordering.
impl Eq for F32 {} // The Eq trait requires no methods to implement it.

// Custom struct to wrap f64 for Eq and Ord
#[derive(Debug, Clone, Copy)]
pub struct F64(f64);

impl AsRef<f64> for F64 {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl PartialEq for F64 {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() {
            other.0.is_nan()
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for F64 {}

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
impl <W> GluinoValueSer<W> for Rc<RefCell<Box<dyn GluinoValueSer<W>>>>
where
    W: Write,
{
    fn serialize(
        &self,
        value: GluinoValue,
        writer: &mut W,
    ) -> Result<usize, GluinoSerializationError> {
        self.borrow().serialize(value, writer)
    }
}

pub trait GluinoValueDe<R>
where
    R: Read,
{
    fn deserialize(&self, reader: &mut R) -> Result<GluinoValue, GluinoDeserializationError>;
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
    InncorrectNumberOfIntegerBytes {
        expect_bytes: usize,
        actual_bytes: usize,
    },
    IncorrectNumberOfFloatingPointBytes {
        expext_bytes: usize,
        actual_bytes: usize,
    },
    UnknownConstSetIndex (u64),
}

impl From<io::Error> for GluinoSerializationError {
    fn from(e: io::Error) -> Self {
        GluinoSerializationError::WriteError(e)
    }
}

pub enum GluinoDeserializationError {
    ReadError(io::Error),
}

impl From<io::Error> for GluinoDeserializationError {
    fn from(e: io::Error) -> Self {
        GluinoDeserializationError::ReadError(e)
    }
}

pub fn get_unit_serialization_function<W>(spec: &CompiledSpec) -> Box<dyn GluinoValueSer<W>>
where
    for<'ser> (dyn GluinoValueSer<W>): 'ser,
    for<'write> W: Write + 'write,
{
    get_unit_serialization_function_internal::<W>(spec, &mut HashMap::new())
}

fn get_unit_serialization_function_internal<W>(
    spec: &CompiledSpec,
    named_unit_sers: &mut HashMap<String, Rc<RefCell<Box<dyn GluinoValueSer<W>>>>>,
) -> Box<dyn GluinoValueSer<W>>
where
    for<'ser> (dyn GluinoValueSer<W>): 'ser,
    for<'write> W: Write + 'write,
{
    match spec.structure() {
        CompiledSpecStructure::Void => Box::new(VoidGluinoValueSer),
        CompiledSpecStructure::Bool => Box::new(NativeSingleSer::<bool>::new()),
        CompiledSpecStructure::Uint(n) => match n {
            0 => Box::new(NativeSingleSer::<u8>::new()),
            1 => Box::new(NativeSingleSer::<u16>::new()),
            2 => Box::new(NativeSingleSer::<u32>::new()),
            3 => Box::new(NativeSingleSer::<u64>::new()),
            4 => Box::new(NativeSingleSer::<u128>::new()),
            _ => {
                let n = n.clone();
                Box::new(BigUintValueSer { n })
            }
        },
        CompiledSpecStructure::Int(n) => match n {
            0 => Box::new(NativeSingleSer::<i8>::new()),
            1 => Box::new(NativeSingleSer::<i16>::new()),
            2 => Box::new(NativeSingleSer::<i32>::new()),
            3 => Box::new(NativeSingleSer::<i64>::new()),
            4 => Box::new(NativeSingleSer::<i128>::new()),
            _ => {
                let n = n.clone();
                Box::new(BigIntValueSer { n })
            }
        },
        CompiledSpecStructure::BinaryFloatingPoint(fmt) => match fmt {
            InterchangeBinaryFloatingPointFormat::Single => Box::new(NativeSingleSer::<F32>::new()),
            InterchangeBinaryFloatingPointFormat::Double => Box::new(NativeSingleSer::<F64>::new()),
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
                let named_ser: Rc<RefCell<Box<dyn GluinoValueSer<W>>>> =
                    Rc::new(RefCell::new(Box::new(VoidGluinoValueSer)));
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
        CompiledSpecStructure::ConstSet(const_spec, const_values) => {
            Box::new(ConstSetSer {
                const_values: const_values.clone(),
                const_ser: get_unit_serialization_function_internal::<W>(const_spec, named_unit_sers),
            })
        }
    }
}

pub fn get_unit_deserialization_function<R>(spec: &CompiledSpec) -> Box<dyn GluinoValueDe<R>>
where
    R: Read,
{
    match spec.structure() {
        CompiledSpecStructure::Void => Box::new(VoidGluinoValueDe),
        CompiledSpecStructure::Bool => Box::new(NativeSingleDe::<bool>::new()),
        CompiledSpecStructure::Uint(n) => {
            match n {
                0 => Box::new(NativeSingleDe::<u8>::new()),
                1 => Box::new(NativeSingleDe::<u16>::new()),
                2 => Box::new(NativeSingleDe::<u32>::new()),
                3 => Box::new(NativeSingleDe::<u64>::new()),
                4 => Box::new(NativeSingleDe::<u128>::new()),
                _ => todo!()
            }
        },
        CompiledSpecStructure::Int(n) => {
            match n {
                0 => Box::new(NativeSingleDe::<i8>::new()),
                1 => Box::new(NativeSingleDe::<i16>::new()),
                2 => Box::new(NativeSingleDe::<i32>::new()),
                3 => Box::new(NativeSingleDe::<i64>::new()),
                4 => Box::new(NativeSingleDe::<i128>::new()),
                _ => todo!()
            }
        },
        CompiledSpecStructure::BinaryFloatingPoint(fmt) => {
            match fmt {
                InterchangeBinaryFloatingPointFormat::Single => Box::new(NativeSingleDe::<F32>::new()),
                InterchangeBinaryFloatingPointFormat::Double => Box::new(NativeSingleDe::<F64>::new()),
                InterchangeBinaryFloatingPointFormat::Half => todo!(),
                InterchangeBinaryFloatingPointFormat::Quadruple => todo!(),
                InterchangeBinaryFloatingPointFormat::Octuple => todo!(),
            }
        },
        CompiledSpecStructure::DecimalFloatingPoint(_) => todo!(),
        CompiledSpecStructure::Decimal(_) => todo!(),
        CompiledSpecStructure::Map { size, key_spec, value_spec } => todo!(),
        CompiledSpecStructure::List { size, value_spec } => todo!(),
        CompiledSpecStructure::String(_, _) => todo!(),
        CompiledSpecStructure::Bytes(_) => todo!(),
        CompiledSpecStructure::Optional(_) => todo!(),
        CompiledSpecStructure::Record { fields, field_to_spec, field_to_index } => todo!(),
        CompiledSpecStructure::Tuple(_) => todo!(),
        CompiledSpecStructure::Enum { variants, variant_to_spec } => todo!(),
        CompiledSpecStructure::Union(_) => todo!(),
        CompiledSpecStructure::Name(_) => todo!(),
        CompiledSpecStructure::ConstSet(_,_) => todo!(),
    }
}

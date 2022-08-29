use std::{ops::Add, fmt::Write, io::Read};

use crate::util::variable_length_encode;

#[derive(Hash, Eq, PartialEq, Clone)]
enum Size {
    Fixed(u64),
    Variable,
}

pub trait GluinoSpecType {
    fn get_spec() -> Spec;
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum Spec {
    Bool,
    Uint(u8),
    Int(u8),
    BinaryFloatingPoint(InterchangeBinaryFloatingPointFormat),
    DecimalFloatingPoint(InterchangeDecimalFloatingPointFormat),
    Decimal {
        scale: u64,
        precision: u64,
    },
    Map {
        key_spec: Box<Spec>,
        value_spec: Box<Spec>,
        size: Size,
    },
    List {
        value_spec: Box<Spec>,
        size: Size,
    },
    String(Size),
    Bytes(Size),
    Optional(Box<Spec>),
    Name {
        name: String,
        spec: Box<Spec>,
    },
    Ref {
        name: String,
    },
    Record(Vec<(String, Spec)>),
    Tuple(Vec<Spec>),
    Enum(Vec<(String, Spec)>),
    Union(Vec<Spec>),
    Void,
}

//core
const BOOL: u8 = 32;
const UINT: u8 = 33;
const NAME: u8 = 34;
const INT: u8 = 35;
const BINARY_FP: u8 = 36;
const DECIMAL_FP: u8 = 37;
const REF: u8 = 38;
const VOID: u8 = 39;
const LIST: u8 = 40;
const MAP: u8 = 41;
const RECORD: u8 = 42;
const ENUM: u8 = 43;
const UNION: u8 = 45;
const DECIMAL: u8 = 46;
const TUPLE: u8 = 47;
const BYTES: u8 = 48;
const OPTIONAL: u8 = 63;

// aliases
const UINT_0: u8 = 0;
const UINT_1: u8 = 1;
const UINT_2: u8 = 2;
const UINT_3: u8 = 3;
const INT_0: u8 = 4;
const INT_1: u8 = 5;
const INT_2: u8 = 6;
const INT_3: u8 = 7;
const SINGLE_FP: u8 = 8;
const DOUBLE_FP: u8 = 9;

impl Spec {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        self.to_bytes_internal(&mut out);
        out
    }

    fn to_bytes_internal(&self, out: &mut Vec<u8>) {
        match self {
            Spec::Bool => out.push(BOOL),
            Spec::Uint(scale) => match scale {
                0 => out.push(UINT_0),
                1 => out.push(UINT_1),
                2 => out.push(UINT_2),
                3 => out.push(UINT_3),
                s => out.extend_from_slice(&[UINT, *s]),
            },
            Spec::Int(scale) => match scale {
                0 => out.push(INT_0),
                1 => out.push(INT_1),
                2 => out.push(INT_2),
                3 => out.push(INT_3),
                s => out.extend_from_slice(&[UINT, *s]),
            },
            Spec::BinaryFloatingPoint(fmt) => match fmt {
                InterchangeBinaryFloatingPointFormat::Single => out.push(SINGLE_FP),
                InterchangeBinaryFloatingPointFormat::Double => out.push(DOUBLE_FP),
                InterchangeBinaryFloatingPointFormat::Half => out.extend_from_slice(&[BINARY_FP, 0]),
                InterchangeBinaryFloatingPointFormat::Quadruple => {
                    out.extend_from_slice(&[BINARY_FP, 3])
                }
                InterchangeBinaryFloatingPointFormat::Octuple => {
                    out.extend_from_slice(&[BINARY_FP, 4])
                }
            },
            Spec::DecimalFloatingPoint(fmt) => match fmt {
                InterchangeDecimalFloatingPointFormat::Dec32 => {
                    out.extend_from_slice(&[DECIMAL_FP, 0])
                }
                InterchangeDecimalFloatingPointFormat::Dec64 => {
                    out.extend_from_slice(&[DECIMAL_FP, 1])
                }
                InterchangeDecimalFloatingPointFormat::Dec128 => {
                    out.extend_from_slice(&[DECIMAL_FP, 2])
                }
            },
            Spec::Decimal { scale, precision } => {
                out.push(DECIMAL);
                variable_length_encode(*scale as u128, out);
                variable_length_encode(*precision as u128, out);
            },
            Spec::Map {
                key_spec,
                value_spec,
                size,
            } => {
                out.push(MAP);
                size_to_bytes(size, out);
                Spec::to_bytes_internal(key_spec, out);
                Spec::to_bytes_internal(value_spec, out);
            },
            Spec::List { value_spec, size } => {
                out.push(LIST);
                size_to_bytes(size, out);
                Spec::to_bytes_internal(value_spec, out);
            },
            Spec::String(_) => todo!(),
            Spec::Bytes(size) => {
                out.push(BYTES);
                size_to_bytes(size, out);
            }
            Spec::Optional(optional_type) => {
                out.push(OPTIONAL);
                Spec::to_bytes_internal(&optional_type, out)
            }
            Spec::Name { name, spec } => {
                out.push(NAME);
                encode_string(name, out);
                Spec::to_bytes_internal(spec, out);
            },
            Spec::Ref { name } => {
                out.push(REF);
                encode_string(name, out);
            },
            Spec::Record(fields) => {
                out.push(RECORD);
                variable_length_encode(fields.len() as u128, out);
                for (name, spec) in fields {
                    encode_string(name, out);
                    Spec::to_bytes_internal(&spec, out);
                }
            },
            Spec::Tuple(fields) => {
                out.push(TUPLE);
                variable_length_encode(fields.len() as u128, out);
                for spec in fields {
                    Spec::to_bytes_internal(&spec, out);
                }
            },
            Spec::Enum(variants) => {
                out.push(ENUM);
                variable_length_encode(variants.len() as u128, out);
                for (name, spec) in variants {
                    encode_string(name, out);
                    Spec::to_bytes_internal(spec, out);
                }
            },
            Spec::Union(variants) => {
                out.push(UNION);
                variable_length_encode(variants.len()  as u128, out);
                for spec in variants {
                    Spec::to_bytes_internal(spec, out)
                }
            },
            Spec::Void => {
                out.push(VOID)
            },
        }
    }

    fn from_bytes<R:Read>(bytes: &mut R) -> Result<Spec, SpecParsingError> {
        Ok(Spec::Void)
    }
}

fn encode_string(string: &String, out: &mut Vec<u8>) {
    let b =  string.as_bytes();
    variable_length_encode(b.len() as u128, out);
    out.extend_from_slice(b);
}


fn size_to_bytes(size: &Size, out: &mut Vec<u8>) {
    match size {
        Size::Fixed(n) => {
            out.push(0);
            variable_length_encode(*n as u128, out)
        },
        Size::Variable => {
            out.push(1);
        },
    }
}

pub enum SpecParsingError {}

#[derive(Hash, Eq, PartialEq, Clone)]
enum InterchangeBinaryFloatingPointFormat {
    Half,
    Single,
    Double,
    Quadruple,
    Octuple,
}

impl InterchangeBinaryFloatingPointFormat {
    fn significand_bits(&self) -> u64 {
        match self {
            InterchangeBinaryFloatingPointFormat::Half => 11,
            InterchangeBinaryFloatingPointFormat::Single => 24,
            InterchangeBinaryFloatingPointFormat::Double => 53,
            InterchangeBinaryFloatingPointFormat::Quadruple => 113,
            InterchangeBinaryFloatingPointFormat::Octuple => 237,
        }
    }

    fn exponent_bits(&self) -> u64 {
        match self {
            InterchangeBinaryFloatingPointFormat::Half => 5,
            InterchangeBinaryFloatingPointFormat::Single => 8,
            InterchangeBinaryFloatingPointFormat::Double => 11,
            InterchangeBinaryFloatingPointFormat::Quadruple => 15,
            InterchangeBinaryFloatingPointFormat::Octuple => 19,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone)]
enum InterchangeDecimalFloatingPointFormat {
    Dec32,
    Dec64,
    Dec128,
}

impl InterchangeDecimalFloatingPointFormat {
    fn significand_bits(&self) -> u64 {
        match self {
            InterchangeDecimalFloatingPointFormat::Dec32 => 7,
            InterchangeDecimalFloatingPointFormat::Dec64 => 16,
            InterchangeDecimalFloatingPointFormat::Dec128 => 34,
        }
    }

    fn decimal_digits(&self) -> u64 {
        match self {
            InterchangeDecimalFloatingPointFormat::Dec32 => 7,
            InterchangeDecimalFloatingPointFormat::Dec64 => 16,
            InterchangeDecimalFloatingPointFormat::Dec128 => 34,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
enum StringEncodingFmt{
    Utf8, //use this one, please
    Utf16,
    Ascii,
}

// pub enum CompiledSpec {}

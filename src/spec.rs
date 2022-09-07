use std::{
    io::Read,
    io::{self, Write}, str::Bytes,
};

use crate::util::variable_length_encode_u64;

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
    String(StringEncodingFmt, Size),
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
const STRING: u8 = 49;
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
const UTF8_STRING: u8 = 10;

impl Spec {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        if let Err(e) = self.to_bytes_internal(&mut out) {
            panic!("{}", e.to_string())
        };
        out
    }

    pub fn write_as_bytes<W: Write>(&self, w: &mut W) -> Result<usize, io::Error> {
        self.to_bytes_internal(w)
    }

    fn to_bytes_internal<W: Write>(&self, out: &mut W) -> Result<usize, io::Error> {
        Ok(match self {
            Spec::Bool => out.write(&[BOOL])?,
            Spec::Uint(scale) => match scale {
                0 => out.write(&[UINT_0])?,
                1 => out.write(&[UINT_1])?,
                2 => out.write(&[UINT_2])?,
                3 => out.write(&[UINT_3])?,
                s => out.write(&[UINT, *s])?,
            },
            Spec::Int(scale) => match scale {
                0 => out.write(&[INT_0])?,
                1 => out.write(&[INT_1])?,
                2 => out.write(&[INT_2])?,
                3 => out.write(&[INT_3])?,
                s => out.write(&[UINT, *s])?,
            },
            Spec::BinaryFloatingPoint(fmt) => match fmt {
                InterchangeBinaryFloatingPointFormat::Single => out.write(&[SINGLE_FP])?,
                InterchangeBinaryFloatingPointFormat::Double => out.write(&[DOUBLE_FP])?,
                InterchangeBinaryFloatingPointFormat::Half => out.write(&[BINARY_FP, 0])?,
                InterchangeBinaryFloatingPointFormat::Quadruple => out.write(&[BINARY_FP, 3])?,
                InterchangeBinaryFloatingPointFormat::Octuple => out.write(&[BINARY_FP, 4])?,
            },
            Spec::DecimalFloatingPoint(fmt) => match fmt {
                InterchangeDecimalFloatingPointFormat::Dec32 => out.write(&[DECIMAL_FP, 0])?,
                InterchangeDecimalFloatingPointFormat::Dec64 => out.write(&[DECIMAL_FP, 1])?,
                InterchangeDecimalFloatingPointFormat::Dec128 => out.write(&[DECIMAL_FP, 2])?,
            },
            Spec::Decimal { scale, precision } => {
                out.write(&[DECIMAL])?
                    + variable_length_encode_u64(*scale, out)?
                    + variable_length_encode_u64(*precision, out)?
            }
            Spec::Map {
                key_spec,
                value_spec,
                size,
            } => {
                out.write(&[MAP])?
                    + size_to_bytes(size, out)?
                    + Spec::to_bytes_internal(key_spec, out)?
                    + Spec::to_bytes_internal(value_spec, out)?
            }
            Spec::List { value_spec, size } => {
                out.write(&[LIST])?
                    + size_to_bytes(size, out)?
                    + Spec::to_bytes_internal(value_spec, out)?
            }
            Spec::String(str_fmt, size) => {
                match str_fmt {
                    StringEncodingFmt::Utf8 => out.write(&[UTF8_STRING]),
                    StringEncodingFmt::Utf16 => out.write(&[STRING, 1]),
                    StringEncodingFmt::Ascii => out.write(&[STRING, 2]),
                }? + size_to_bytes(size, out)?
            }
            Spec::Bytes(size) => out.write(&[BYTES])? + size_to_bytes(size, out)?,
            Spec::Optional(optional_type) => {
                out.write(&[OPTIONAL])? + Spec::to_bytes_internal(&optional_type, out)?
            }
            Spec::Name { name, spec } => {
                out.write(&[NAME])?
                    + encode_string(name, out)?
                    + Spec::to_bytes_internal(spec, out)?
            }
            Spec::Ref { name } => out.write(&[REF])? + encode_string(name, out)?,
            Spec::Record(fields) => {
                out.write(&[RECORD])?
                    + variable_length_encode_u64(fields.len() as u64, out)?
                    + fields
                        .iter()
                        .map(|(name, spec)| {
                            combine(
                                encode_string(name, out),
                                Spec::to_bytes_internal(&spec, out),
                            )
                        })
                        .fold(Ok(0usize), combine)?
            }
            Spec::Tuple(fields) => {
                out.write(&[TUPLE])?
                    + variable_length_encode_u64(fields.len() as u64, out)?
                    + fields
                        .iter()
                        .map(|spec| Spec::to_bytes_internal(&spec, out))
                        .fold(Ok(0usize), combine)?
            }
            Spec::Enum(variants) => {
                out.write(&[ENUM])?
                    + variable_length_encode_u64(variants.len() as u64, out)?
                    + variants
                        .iter()
                        .map(|(name, spec)| {
                            combine(encode_string(name, out), Spec::to_bytes_internal(spec, out))
                        })
                        .fold(Ok(0usize), combine)?
            }
            Spec::Union(variants) => {
                out.write(&[UNION])?
                    + variable_length_encode_u64(variants.len() as u64, out)?
                    + variants
                        .iter()
                        .map(|spec| Spec::to_bytes_internal(spec, out))
                        .fold(Ok(0usize), combine)?
            }
            Spec::Void => out.write(&[VOID])?,
        })
    }

    pub fn read_as_bytes<R: Read>(input: &mut R) -> Result<Spec, SpecParsingError> {
        let b =  input.bytes();
        todo!()
    }
}

fn encode_string<W: Write>(string: &String, out: &mut W) -> Result<usize, io::Error> {
    let b = string.as_bytes();
    combine(
        variable_length_encode_u64(b.len() as u64, out),
        out.write(b),
    )
}

fn size_to_bytes<W: Write>(size: &Size, out: &mut W) -> Result<usize, io::Error> {
    match size {
        Size::Fixed(n) => combine(out.write(&[0]), variable_length_encode_u64(*n, out)),
        Size::Variable => out.write(&[1]),
    }
}

pub enum SpecParsingError {
    ReadError(io::Error),
    UnexpectedEndOfBytes,
}

impl From<io::Error> for SpecParsingError {
    fn from(e: io::Error) -> Self {
        SpecParsingError::ReadError(e)
    }
}

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

#[derive(Debug, Hash, Eq, PartialEq, Clone, Default)]
enum StringEncodingFmt {
    #[default]
    Utf8, //use this one, please
    Utf16,
    Ascii,
}

impl StringEncodingFmt {}

fn combine(a: Result<usize, io::Error>, b: Result<usize, io::Error>) -> Result<usize, io::Error> {
    match (a.as_ref(), b.as_ref()) {
        (Ok(i), Ok(j)) => Ok(i + j),
        (Err(_), _) => a,
        (_, Err(_)) => b,
    }
}

// pub enum CompiledSpec {}

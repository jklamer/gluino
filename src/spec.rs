use core::slice;
use gc::{Finalize, Trace};
use std::{
    io::Read,
    io::{self, Write},
};
use strum_macros::{EnumDiscriminants, EnumIter};

use crate::{
    compiled_spec::{CompiledSpec, SpecCompileError},
    util::{
        self, variable_length_decode_u64, variable_length_encode_u64, VariableLengthDecodingError,
        WriteAllReturnSize,
    },
};

#[derive(Debug, Hash, Eq, PartialEq, Clone, EnumDiscriminants)]
#[strum_discriminants(name(SpecKind))]
#[strum_discriminants(derive(EnumIter))]
pub enum Spec {
    Bool,
    Uint(u8),
    Int(u8),
    BinaryFloatingPoint(InterchangeBinaryFloatingPointFormat),
    DecimalFloatingPoint(InterchangeDecimalFloatingPointFormat),
    Decimal {
        precision: u64,
        scale: u64,
    },
    Map {
        size: Size,
        key_spec: Box<Spec>,
        value_spec: Box<Spec>,
    },
    List {
        size: Size,
        value_spec: Box<Spec>,
    },
    String(Size, StringEncodingFmt),
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
    ConstSet(Box<Spec>, Vec<Vec<u8>>),
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

// never used  ( except for testing)
const NEVER_USED: u8 = 0xFF;

impl Spec {
    pub fn compile(self) -> Result<CompiledSpec, SpecCompileError> {
        CompiledSpec::compile(self)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        if let Err(e) = self.to_bytes_internal(&mut out) {
            panic!("{}", e.to_string())
        };
        out
    }

    pub(crate) fn to_longform_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        if let Err(e) = self.to_longform_bytes_internal(&mut out) {
            panic!("{}", e.to_string())
        };
        out
    }

    pub fn write_as_bytes<W: Write>(&self, w: &mut W) -> Result<usize, io::Error> {
        self.to_bytes_internal(w)
    }

    fn to_bytes_internal<W: Write>(&self, out: &mut W) -> Result<usize, io::Error> {
        Ok(match self {
            Spec::Bool => out.write_all_size(&[BOOL])?,
            Spec::Uint(scale) => match scale {
                0 => out.write_all_size(&[UINT_0])?,
                1 => out.write_all_size(&[UINT_1])?,
                2 => out.write_all_size(&[UINT_2])?,
                3 => out.write_all_size(&[UINT_3])?,
                s => out.write_all_size(&[UINT, *s])?,
            },
            Spec::Int(scale) => match scale {
                0 => out.write_all_size(&[INT_0])?,
                1 => out.write_all_size(&[INT_1])?,
                2 => out.write_all_size(&[INT_2])?,
                3 => out.write_all_size(&[INT_3])?,
                s => out.write_all_size(&[INT, *s])?,
            },
            Spec::BinaryFloatingPoint(fmt) => match fmt {
                InterchangeBinaryFloatingPointFormat::Single => out.write_all_size(&[SINGLE_FP])?,
                InterchangeBinaryFloatingPointFormat::Double => out.write_all_size(&[DOUBLE_FP])?,
                fmt => out.write_all_size(&[BINARY_FP])? + fmt.encode(out)?,
            },
            Spec::DecimalFloatingPoint(fmt) => {
                out.write_all_size(&[DECIMAL_FP])? + fmt.encode(out)?
            }
            Spec::Decimal { precision, scale } => {
                out.write_all_size(&[DECIMAL])?
                    + variable_length_encode_u64(*precision, out)?
                    + variable_length_encode_u64(*scale, out)?
            }
            Spec::Map {
                size,
                key_spec,
                value_spec,
            } => {
                out.write_all_size(&[MAP])?
                    + size.encode(out)?
                    + Spec::to_bytes_internal(key_spec, out)?
                    + Spec::to_bytes_internal(value_spec, out)?
            }
            Spec::List { value_spec, size } => {
                out.write_all_size(&[LIST])?
                    + size.encode(out)?
                    + Spec::to_bytes_internal(value_spec, out)?
            }
            Spec::String(size, str_fmt) => {
                if matches!(size, Size::Variable) && matches!(str_fmt, StringEncodingFmt::Utf8) {
                    out.write_all_size(&[UTF8_STRING])?
                } else {
                    out.write_all_size(&[STRING])? + size.encode(out)? + str_fmt.encode(out)?
                }
            }
            Spec::Bytes(size) => out.write_all_size(&[BYTES])? + size.encode(out)?,
            Spec::Optional(optional_type) => {
                out.write_all_size(&[OPTIONAL])? + Spec::to_bytes_internal(&optional_type, out)?
            }
            Spec::Name { name, spec } => {
                out.write_all_size(&[NAME])?
                    + encode_string_utf8(name, out)?
                    + Spec::to_bytes_internal(spec, out)?
            }
            Spec::Ref { name } => out.write_all_size(&[REF])? + encode_string_utf8(name, out)?,
            Spec::Record(fields) => {
                out.write_all_size(&[RECORD])?
                    + variable_length_encode_u64(fields.len() as u64, out)?
                    + fields
                        .iter()
                        .map(|(name, spec)| {
                            combine(
                                encode_string_utf8(name, out),
                                Spec::to_bytes_internal(&spec, out),
                            )
                        })
                        .fold(Ok(0usize), combine)?
            }
            Spec::Tuple(fields) => {
                out.write_all_size(&[TUPLE])?
                    + variable_length_encode_u64(fields.len() as u64, out)?
                    + fields
                        .iter()
                        .map(|spec| Spec::to_bytes_internal(&spec, out))
                        .fold(Ok(0usize), combine)?
            }
            Spec::Enum(variants) => {
                out.write_all_size(&[ENUM])?
                    + variable_length_encode_u64(variants.len() as u64, out)?
                    + variants
                        .iter()
                        .map(|(name, spec)| {
                            combine(
                                encode_string_utf8(name, out),
                                Spec::to_bytes_internal(spec, out),
                            )
                        })
                        .fold(Ok(0usize), combine)?
            }
            Spec::Union(variants) => {
                out.write_all_size(&[UNION])?
                    + variable_length_encode_u64(variants.len() as u64, out)?
                    + variants
                        .iter()
                        .map(|spec| Spec::to_bytes_internal(spec, out))
                        .fold(Ok(0usize), combine)?
            }
            Spec::Void => out.write_all_size(&[VOID])?,
        })
    }

    pub fn read_from_bytes<R: Read>(input: &mut R) -> Result<Spec, SpecParsingError> {
        match next_byte(input)? {
            BOOL => Ok(Spec::Bool),
            VOID => Ok(Spec::Void),
            UINT => Ok(Spec::Uint(next_byte(input)?)),
            INT => Ok(Spec::Int(next_byte(input)?)),
            NAME => {
                let name = decode_utf8_string(input)?;
                let spec = Spec::read_from_bytes(input)?.into();
                Ok(Spec::Name { name, spec })
            }
            REF => Ok(Spec::Ref {
                name: decode_utf8_string(input)?,
            }),
            BINARY_FP => Ok(Spec::BinaryFloatingPoint(
                InterchangeBinaryFloatingPointFormat::decode(input)?,
            )),
            DECIMAL_FP => Ok(Spec::DecimalFloatingPoint(
                InterchangeDecimalFloatingPointFormat::decode(input)?,
            )),
            LIST => {
                let size = Size::decode(input)?;
                let value_spec = Spec::read_from_bytes(input)?.into();
                Ok(Spec::List { size, value_spec })
            }
            MAP => {
                let size = Size::decode(input)?;
                let key_spec = Spec::read_from_bytes(input)?.into();
                let value_spec = Spec::read_from_bytes(input)?.into();
                Ok(Spec::Map {
                    size,
                    key_spec,
                    value_spec,
                })
            }
            DECIMAL => {
                let precision = decode_u64(input)?;
                let scale = decode_u64(input)?;
                Ok(Spec::Decimal { precision, scale })
            }
            BYTES => {
                let size = Size::decode(input)?;
                Ok(Spec::Bytes(size))
            }
            STRING => {
                let size = Size::decode(input)?;
                let str_fmt = StringEncodingFmt::decode(input)?;
                Ok(Spec::String(size, str_fmt))
            }
            OPTIONAL => Ok(Spec::Optional(Spec::read_from_bytes(input)?.into())),
            RECORD => {
                let n = decode_u64(input)?;
                let mut v = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    v.push((decode_utf8_string(input)?, Spec::read_from_bytes(input)?));
                }
                Ok(Spec::Record(v))
            }
            TUPLE => {
                let n = decode_u64(input)?;
                let mut v = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    v.push(Spec::read_from_bytes(input)?);
                }
                Ok(Spec::Tuple(v))
            }
            ENUM => {
                let n = decode_u64(input)?;
                let mut v = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    v.push((decode_utf8_string(input)?, Spec::read_from_bytes(input)?));
                }
                Ok(Spec::Enum(v))
            }
            UNION => {
                let n = decode_u64(input)?;
                let mut v = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    v.push(Spec::read_from_bytes(input)?);
                }
                Ok(Spec::Union(v))
            }
            // aliases
            UINT_0 => Ok(Spec::Uint(0)),
            UINT_1 => Ok(Spec::Uint(1)),
            UINT_2 => Ok(Spec::Uint(2)),
            UINT_3 => Ok(Spec::Uint(3)),
            INT_0 => Ok(Spec::Int(0)),
            INT_1 => Ok(Spec::Int(1)),
            INT_2 => Ok(Spec::Int(2)),
            INT_3 => Ok(Spec::Int(3)),
            SINGLE_FP => Ok(Spec::BinaryFloatingPoint(
                InterchangeBinaryFloatingPointFormat::Single,
            )),
            DOUBLE_FP => Ok(Spec::BinaryFloatingPoint(
                InterchangeBinaryFloatingPointFormat::Double,
            )),
            UTF8_STRING => Ok(Spec::String(Size::Variable, StringEncodingFmt::Utf8)),
            flag => Err(SpecParsingError::UnknownSpecFlag(flag)),
        }
    }

    /// To longform bytes creates serialized view of spec without use of alias bytes for compression
    /// This byte representation is used for identification purposes
    pub(crate) fn to_longform_bytes_internal<W: Write>(
        &self,
        out: &mut W,
    ) -> Result<usize, io::Error> {
        Ok(match self {
            Spec::Bool => out.write_all_size(&[BOOL])?,
            Spec::Uint(scale) => out.write_all_size(&[UINT, *scale])?,
            Spec::Int(scale) => out.write_all_size(&[INT, *scale])?,
            Spec::BinaryFloatingPoint(fmt) => {
                out.write_all_size(&[BINARY_FP])? + fmt.encode(out)?
            }
            Spec::DecimalFloatingPoint(fmt) => {
                out.write_all_size(&[DECIMAL_FP])? + fmt.encode(out)?
            }
            Spec::Decimal { precision, scale } => {
                out.write_all_size(&[DECIMAL])?
                    + variable_length_encode_u64(*precision, out)?
                    + variable_length_encode_u64(*scale, out)?
            }
            Spec::Map {
                size,
                key_spec,
                value_spec,
            } => {
                out.write_all_size(&[MAP])?
                    + size.encode(out)?
                    + Spec::to_bytes_internal(key_spec, out)?
                    + Spec::to_bytes_internal(value_spec, out)?
            }
            Spec::List { value_spec, size } => {
                out.write_all_size(&[LIST])?
                    + size.encode(out)?
                    + Spec::to_bytes_internal(value_spec, out)?
            }
            Spec::String(size, str_fmt) => {
                if matches!(size, Size::Variable) && matches!(str_fmt, StringEncodingFmt::Utf8) {
                    out.write_all_size(&[UTF8_STRING])?
                } else {
                    out.write_all_size(&[STRING])? + size.encode(out)? + str_fmt.encode(out)?
                }
            }
            Spec::Bytes(size) => out.write_all_size(&[BYTES])? + size.encode(out)?,
            Spec::Optional(optional_type) => {
                out.write_all_size(&[OPTIONAL])? + Spec::to_bytes_internal(&optional_type, out)?
            }
            Spec::Name { name, spec } => {
                out.write_all_size(&[NAME])?
                    + encode_string_utf8(name, out)?
                    + Spec::to_bytes_internal(spec, out)?
            }
            Spec::Ref { name } => out.write_all_size(&[REF])? + encode_string_utf8(name, out)?,
            Spec::Record(fields) => {
                out.write_all_size(&[RECORD])?
                    + variable_length_encode_u64(fields.len() as u64, out)?
                    + fields
                        .iter()
                        .map(|(name, spec)| {
                            combine(
                                encode_string_utf8(name, out),
                                Spec::to_bytes_internal(&spec, out),
                            )
                        })
                        .fold(Ok(0usize), combine)?
            }
            Spec::Tuple(fields) => {
                out.write_all_size(&[TUPLE])?
                    + variable_length_encode_u64(fields.len() as u64, out)?
                    + fields
                        .iter()
                        .map(|spec| Spec::to_bytes_internal(&spec, out))
                        .fold(Ok(0usize), combine)?
            }
            Spec::Enum(variants) => {
                out.write_all_size(&[ENUM])?
                    + variable_length_encode_u64(variants.len() as u64, out)?
                    + variants
                        .iter()
                        .map(|(name, spec)| {
                            combine(
                                encode_string_utf8(name, out),
                                Spec::to_bytes_internal(spec, out),
                            )
                        })
                        .fold(Ok(0usize), combine)?
            }
            Spec::Union(variants) => {
                out.write_all_size(&[UNION])?
                    + variable_length_encode_u64(variants.len() as u64, out)?
                    + variants
                        .iter()
                        .map(|spec| Spec::to_bytes_internal(spec, out))
                        .fold(Ok(0usize), combine)?
            }
            Spec::Void => out.write_all_size(&[VOID])?,
            Spec::ConstSet(_, _) => {}
        })
    }
}

#[inline]
fn encode_string_utf8<W: Write>(string: &String, out: &mut W) -> Result<usize, io::Error> {
    let b = string.as_bytes();
    Ok(variable_length_encode_u64(b.len() as u64, out)? + out.write_all_size(b)?)
}

fn decode_utf8_string<R: Read>(input: &mut R) -> Result<String, SpecParsingError> {
    let n = decode_u64(input)?;
    let mut s = String::with_capacity(n as usize);
    let n_actual = input.take(n).read_to_string(&mut s)?;
    if (n_actual as u64) < n {
        Err(SpecParsingError::UnexpectedEndOfBytes)
    } else {
        Ok(s)
    }
}

fn decode_u64<R: Read>(input: &mut R) -> Result<u64, SpecParsingError> {
    match variable_length_decode_u64(input)? {
        util::VariableLengthResult::Respresentable(n) => Ok(n),
        util::VariableLengthResult::Unrepresentable(v) => {
            return Err(SpecParsingError::IntegerOverflowVariableLengthDecodingError(v))
        }
    }
}

#[inline]
fn next_byte<R: Read>(input: &mut R) -> Result<u8, SpecParsingError> {
    let mut flag: u8 = 255;
    if 0usize == input.read(slice::from_mut(&mut flag))? {
        Err(SpecParsingError::UnexpectedEndOfBytes)
    } else {
        Ok(flag)
    }
}

#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(name(SpecParsingErrorKind))]
#[strum_discriminants(derive(EnumIter))]
pub enum SpecParsingError {
    ReadError(io::Error),
    UnexpectedEndOfBytes,
    UnknownSpecFlag(u8),
    UnknownBinaryFormatFlag(u8),
    UnknownDecimalFormatFlag(u8),
    UnknownStringFormatFlag(u8),
    UnknownSizeFormatFlag(u8),
    IntegerOverflowVariableLengthDecodingError(Vec<u8>),
}

impl From<io::Error> for SpecParsingError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::UnexpectedEof => SpecParsingError::UnexpectedEndOfBytes,
            _ => SpecParsingError::ReadError(e),
        }
    }
}

impl From<VariableLengthDecodingError> for SpecParsingError {
    fn from(e: VariableLengthDecodingError) -> Self {
        match e {
            VariableLengthDecodingError::IncompleteVariableLengthEncoding => {
                SpecParsingError::UnexpectedEndOfBytes
            }
            VariableLengthDecodingError::IoError(e) => SpecParsingError::ReadError(e),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Size {
    Variable,
    Fixed(u64),
    Range(SizeRange),
    // Inclusive
    GreaterThan(u64),
    // Exclusive
    LessThan(u64),
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct SizeRange {
    pub start: u64,
    pub end: u64,
}

impl Size {
    #[inline]
    pub(crate) fn encode<W: Write>(&self, out: &mut W) -> Result<usize, io::Error> {
        match self {
            Size::Variable => out.write_all_size(&[0]),
            Size::Fixed(n) => combine(
                out.write_all_size(&[1]),
                variable_length_encode_u64(*n, out),
            ),
            Size::Range(r) => combine(
                combine(
                    out.write_all_size(&[2]),
                    variable_length_encode_u64(r.start, out),
                ),
                variable_length_encode_u64(r.end, out),
            ),
        }
    }

    #[inline]
    pub(crate) fn decode<R: Read>(input: &mut R) -> Result<Size, SpecParsingError> {
        match next_byte(input)? {
            0 => Ok(Self::Variable),
            1 => Ok(Self::Fixed(decode_u64(input)?)),
            2 => Ok(Self::Range(SizeRange {
                start: decode_u64(input)?,
                end: decode_u64(input)?,
            })),
            b => Err(SpecParsingError::UnknownSizeFormatFlag(b)),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, EnumIter, Trace, Finalize)]
pub enum InterchangeBinaryFloatingPointFormat {
    Half,
    Single,
    Double,
    Quadruple,
    Octuple,
}

impl InterchangeBinaryFloatingPointFormat {
    pub fn significand_bits(&self) -> u64 {
        match self {
            InterchangeBinaryFloatingPointFormat::Half => 11,
            InterchangeBinaryFloatingPointFormat::Single => 24,
            InterchangeBinaryFloatingPointFormat::Double => 53,
            InterchangeBinaryFloatingPointFormat::Quadruple => 113,
            InterchangeBinaryFloatingPointFormat::Octuple => 237,
        }
    }

    pub fn exponent_bits(&self) -> u64 {
        match self {
            InterchangeBinaryFloatingPointFormat::Half => 5,
            InterchangeBinaryFloatingPointFormat::Single => 8,
            InterchangeBinaryFloatingPointFormat::Double => 11,
            InterchangeBinaryFloatingPointFormat::Quadruple => 15,
            InterchangeBinaryFloatingPointFormat::Octuple => 19,
        }
    }

    #[inline]
    pub(crate) fn encode<W: Write>(&self, out: &mut W) -> Result<usize, io::Error> {
        match self {
            InterchangeBinaryFloatingPointFormat::Half => out.write_all_size(&[0]),
            InterchangeBinaryFloatingPointFormat::Single => out.write_all_size(&[1]),
            InterchangeBinaryFloatingPointFormat::Double => out.write_all_size(&[2]),
            InterchangeBinaryFloatingPointFormat::Quadruple => out.write_all_size(&[3]),
            InterchangeBinaryFloatingPointFormat::Octuple => out.write_all_size(&[4]),
        }
    }

    #[inline]
    pub(crate) fn decode<R: Read>(
        input: &mut R,
    ) -> Result<InterchangeBinaryFloatingPointFormat, SpecParsingError> {
        Ok(match next_byte(input)? {
            0 => InterchangeBinaryFloatingPointFormat::Half,
            1 => InterchangeBinaryFloatingPointFormat::Single,
            2 => InterchangeBinaryFloatingPointFormat::Double,
            3 => InterchangeBinaryFloatingPointFormat::Quadruple,
            4 => InterchangeBinaryFloatingPointFormat::Octuple,
            b => return Err(SpecParsingError::UnknownBinaryFormatFlag(b)),
        })
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, EnumIter, Trace, Finalize)]
pub enum InterchangeDecimalFloatingPointFormat {
    Dec32,
    Dec64,
    Dec128,
}

impl InterchangeDecimalFloatingPointFormat {
    pub fn significand_bits(&self) -> u64 {
        match self {
            InterchangeDecimalFloatingPointFormat::Dec32 => 7,
            InterchangeDecimalFloatingPointFormat::Dec64 => 16,
            InterchangeDecimalFloatingPointFormat::Dec128 => 34,
        }
    }

    pub fn decimal_digits(&self) -> u64 {
        match self {
            InterchangeDecimalFloatingPointFormat::Dec32 => 7,
            InterchangeDecimalFloatingPointFormat::Dec64 => 16,
            InterchangeDecimalFloatingPointFormat::Dec128 => 34,
        }
    }

    pub(crate) fn minimum_byes_needed(&self) -> usize {
        match self {
            InterchangeDecimalFloatingPointFormat::Dec32 => 2,
            InterchangeDecimalFloatingPointFormat::Dec64 => 4,
            InterchangeDecimalFloatingPointFormat::Dec128 => 8,
        }
    }

    #[inline]
    pub(crate) fn encode<W: Write>(&self, out: &mut W) -> Result<usize, io::Error> {
        match self {
            InterchangeDecimalFloatingPointFormat::Dec32 => out.write_all_size(&[0]),
            InterchangeDecimalFloatingPointFormat::Dec64 => out.write_all_size(&[1]),
            InterchangeDecimalFloatingPointFormat::Dec128 => out.write_all_size(&[2]),
        }
    }

    #[inline]
    pub(crate) fn decode<R: Read>(
        input: &mut R,
    ) -> Result<InterchangeDecimalFloatingPointFormat, SpecParsingError> {
        Ok(match next_byte(input)? {
            0 => InterchangeDecimalFloatingPointFormat::Dec32,
            1 => InterchangeDecimalFloatingPointFormat::Dec64,
            2 => InterchangeDecimalFloatingPointFormat::Dec128,
            b => return Err(SpecParsingError::UnknownDecimalFormatFlag(b)),
        })
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Default, EnumIter, Trace, Finalize)]
pub enum StringEncodingFmt {
    #[default]
    Utf8, //use this one, please
    Utf16,
    Ascii,
}

impl StringEncodingFmt {
    #[inline]
    pub(crate) fn encode<W: Write>(&self, out: &mut W) -> Result<usize, io::Error> {
        match self {
            StringEncodingFmt::Utf8 => out.write_all_size(&[0]),
            StringEncodingFmt::Utf16 => out.write_all_size(&[1]),
            StringEncodingFmt::Ascii => out.write_all_size(&[2]),
        }
    }

    #[inline]
    pub(crate) fn decode<R: Read>(input: &mut R) -> Result<StringEncodingFmt, SpecParsingError> {
        Ok(match next_byte(input)? {
            0 => StringEncodingFmt::Utf8,
            1 => StringEncodingFmt::Utf16,
            2 => StringEncodingFmt::Ascii,
            b => return Err(SpecParsingError::UnknownStringFormatFlag(b)),
        })
    }
}

pub(crate) fn combine<E>(a: Result<usize, E>, b: Result<usize, E>) -> Result<usize, E> {
    match (a.as_ref(), b.as_ref()) {
        (Ok(i), Ok(j)) => Ok(i + j),
        (Err(_), _) => a,
        (_, Err(_)) => b,
    }
}

#[cfg(test)]
mod tests {

    use strum::IntoEnumIterator;

    use crate::test_utils::get_all_kinds_spec;

    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_serde() {
        fn test_spec_serde(spec: Spec) {
            assert_eq!(
                spec,
                Spec::read_from_bytes(&mut Cursor::new(spec.to_bytes()))
                    .expect(format!("Unable to read {:?}", spec).as_str())
            );
        }
        for spec in get_all_kinds_spec() {
            test_spec_serde(spec);
        }
    }

    #[test]
    fn test_longform_serde() {
        fn test_spec_longform_serde(spec: Spec) {
            assert_eq!(
                spec,
                Spec::read_from_bytes(&mut Cursor::new(spec.to_longform_bytes()))
                    .expect(format!("Unable to read {:?}", spec).as_str())
            );
        }
        for spec in get_all_kinds_spec() {
            test_spec_longform_serde(spec);
        }
    }

    #[test]
    fn test_write_size() {
        fn test_spec_write_size(spec: Spec) {
            let mut v = Vec::new();
            let reported_size = spec
                .write_as_bytes(&mut v)
                .expect(format!("Unable to write to bytes. Spec: {}", stringify!($spec)).as_str());
            assert_eq!(v.len(), reported_size);
        }
        for spec in get_all_kinds_spec() {
            test_spec_write_size(spec);
        }
    }

    #[test]
    fn test_eof_deserialization() {
        fn test_eof_exception(spec: Spec) {
            let mut v = Vec::new();
            spec.write_as_bytes(&mut v)
                .expect(format!("Unable to write to bytes. Spec: {:?}", spec).as_str());
            v.truncate(v.len() / 2);
            let res: Result<Spec, SpecParsingError> = Spec::read_from_bytes(&mut Cursor::new(&v));
            if let SpecParsingError::UnexpectedEndOfBytes =
                res.expect_err("Unexpectedly parsed bytes to Spec")
            {
                assert!(true);
            } else {
                assert!(
                    false,
                    "EOF error expected for spec: {:?} with bytes {:?}",
                    spec, v
                );
            }
        }

        for spec in get_all_kinds_spec() {
            test_eof_exception(spec);
        }
    }

    #[test]
    fn test_spec_parsing_errors() {
        for parsing_error_kind in SpecParsingErrorKind::iter() {
            match parsing_error_kind {
                SpecParsingErrorKind::ReadError => {
                    //todo find good way to test.
                    Vec::<Result<Spec, SpecParsingError>>::with_capacity(0)
                }
                SpecParsingErrorKind::UnexpectedEndOfBytes => {
                    //covered in own test case
                    Vec::<Result<Spec, SpecParsingError>>::with_capacity(0)
                }
                SpecParsingErrorKind::UnknownSpecFlag => {
                    vec![Spec::read_from_bytes(&mut Cursor::new(&[NEVER_USED]))]
                }
                SpecParsingErrorKind::UnknownBinaryFormatFlag => {
                    vec![Spec::read_from_bytes(&mut Cursor::new(&[
                        BINARY_FP, NEVER_USED,
                    ]))]
                }
                SpecParsingErrorKind::UnknownDecimalFormatFlag => {
                    vec![Spec::read_from_bytes(&mut Cursor::new(&[
                        DECIMAL_FP, NEVER_USED,
                    ]))]
                }
                SpecParsingErrorKind::UnknownStringFormatFlag => {
                    vec![Spec::read_from_bytes(&mut Cursor::new(&[
                        STRING, 0x00, NEVER_USED,
                    ]))]
                }
                SpecParsingErrorKind::UnknownSizeFormatFlag => {
                    vec![Spec::read_from_bytes(&mut Cursor::new(&[
                        BYTES, NEVER_USED,
                    ]))]
                }
                SpecParsingErrorKind::IntegerOverflowVariableLengthDecodingError => {
                    vec![
                        //way too big a size
                        Spec::read_from_bytes(&mut Cursor::new(&[
                            BYTES, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                            0xFF, 0xFF, 0x01,
                        ])),
                    ]
                }
            }
            .into_iter()
            .map(|res| res.map_err(SpecParsingErrorKind::from))
            .for_each(|res| match res {
                Ok(unexpected_spec) => {
                    assert!(false, "Unexpectedly parsed into {:?}", unexpected_spec)
                }
                Err(e) => {
                    assert_eq!(e, parsing_error_kind, "Unexpeted Error Kind")
                }
            })
        }
    }
}

use std::ops::Add;

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
        let mut out = Vec::with_capacity(128);
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
                s => out.copy_from_slice(&[UINT, *s]),
            },
            Spec::Int(scale) => match scale {
                0 => out.push(INT_0),
                1 => out.push(INT_1),
                2 => out.push(INT_2),
                3 => out.push(INT_3),
                s => out.copy_from_slice(&[UINT, *s]),
            },
            Spec::BinaryFloatingPoint(fmt) => match fmt {
                InterchangeBinaryFloatingPointFormat::Single => out.push(SINGLE_FP),
                InterchangeBinaryFloatingPointFormat::Double => out.push(DOUBLE_FP),
                InterchangeBinaryFloatingPointFormat::Half => out.copy_from_slice(&[BINARY_FP, 0]),
                InterchangeBinaryFloatingPointFormat::Quadruple => {
                    out.copy_from_slice(&[BINARY_FP, 3])
                }
                InterchangeBinaryFloatingPointFormat::Octuple => {
                    out.copy_from_slice(&[BINARY_FP, 4])
                }
            },
            Spec::DecimalFloatingPoint(fmt) => match fmt {
                InterchangeDecimalFloatingPointFormat::Dec32 => {
                    out.copy_from_slice(&[DECIMAL_FP, 0])
                }
                InterchangeDecimalFloatingPointFormat::Dec64 => {
                    out.copy_from_slice(&[DECIMAL_FP, 1])
                }
                InterchangeDecimalFloatingPointFormat::Dec128 => {
                    out.copy_from_slice(&[DECIMAL_FP, 2])
                }
            },
            Spec::Decimal { scale, precision } => todo!(),
            Spec::Map {
                key_spec,
                value_spec,
                size,
            } => todo!(),
            Spec::List { value_spec, size } => todo!(),
            Spec::String(_) => todo!(),
            Spec::Bytes(_) => {}
            Spec::Optional(optional_type) => {
                out.push(OPTIONAL);
                Spec::to_bytes_internal(&optional_type, out)
            }
            Spec::Name { name, spec } => todo!(),
            Spec::Ref { name } => todo!(),
            Spec::Record(_) => todo!(),
            Spec::Tuple(_) => todo!(),
            Spec::Enum(_) => todo!(),
            Spec::Union(_) => todo!(),
            Spec::Void => todo!(),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Spec, SpecParsingError> {
        Ok(Spec::Void)
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

// pub enum CompiledSpec {}

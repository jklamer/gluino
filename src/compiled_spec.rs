use crate::spec::{
    InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size, Spec,
    StringEncodingFmt,
};
use std::{collections::HashMap, sync::Arc};
use strum::{EnumDiscriminants, EnumIter};

#[derive(Debug, Eq, PartialEq, Clone, EnumDiscriminants)]
#[strum_discriminants(name(SpecKind))]
#[strum_discriminants(derive(EnumIter))]
pub enum CompiledSpec {
    Void,
    Bool,
    Uint(u8),
    Int(u8),
    BinaryFloatingPoint(InterchangeBinaryFloatingPointFormat),
    DecimalFloatingPoint(InterchangeDecimalFloatingPointFormat),
    Decimal(DecimalFmt),
    Map {
        size: Size,
        key_spec: Box<CompiledSpec>,
        value_spec: Box<CompiledSpec>,
    },
    List {
        size: Size,
        value_spec: Box<CompiledSpec>,
    },
    String(Size, StringEncodingFmt),
    Bytes(Size),
    Optional(Box<CompiledSpec>),
    Name {
        name: String,
        spec: Arc<CompiledSpec>,
    },
    Record {
        fields: Vec<String>,
        field_to_spec: HashMap<String, CompiledSpec>,
        field_to_index: HashMap<String, usize>,
    },
    Tuple(Vec<CompiledSpec>),
    Enum {
        variants: Vec<String>,
        variant_to_spec: HashMap<String, CompiledSpec>,
    },
    Union(Vec<CompiledSpec>),
}

pub enum SpecCompileError {
    DuplicateName(String),
    UndefinedName(String),
    DuplicateRecordFieldNames(Vec<String>),
    IllegalDecimalFmt,
}

impl From<IllegalDecimalFmt> for SpecCompileError {
    fn from(_: IllegalDecimalFmt) -> Self {
        SpecCompileError::IllegalDecimalFmt
    }
}

impl CompiledSpec {
    fn compile(
        spec: Spec,
        context: &mut HashMap<String, Arc<CompiledSpec>>,
    ) -> Result<CompiledSpec, SpecCompileError> {
        match spec {
            Spec::Bool => Ok(CompiledSpec::Bool),
            Spec::Uint(n) => Ok(CompiledSpec::Uint(n)),
            Spec::Int(n) => Ok(CompiledSpec::Int(n)),
            Spec::BinaryFloatingPoint(fmt) => Ok(CompiledSpec::BinaryFloatingPoint(fmt)),
            Spec::DecimalFloatingPoint(fmt) => Ok(CompiledSpec::DecimalFloatingPoint(fmt)),
            Spec::Decimal { precision, scale } => {
                Ok(CompiledSpec::Decimal(DecimalFmt::new(precision, scale)?))
            }
            Spec::Map {
                size,
                key_spec,
                value_spec,
            } => Ok(CompiledSpec::Map {
                size,
                key_spec: box_compile(key_spec, context)?,
                value_spec: box_compile(value_spec, context)?,
            }),
            Spec::List { size, value_spec } => Ok(CompiledSpec::List {
                size,
                value_spec: box_compile(value_spec, context)?,
            }),
            Spec::String(size, fmt) => Ok(CompiledSpec::String(size, fmt)),
            Spec::Bytes(size) => Ok(CompiledSpec::Bytes(size)),
            Spec::Optional(spec) => Ok(CompiledSpec::Optional(box_compile(spec, context)?)),
            Spec::Name { name, spec } => {
                if context.contains_key(&name) {
                    Err(SpecCompileError::DuplicateName(name))
                } else {
                    let spec_ptr = Arc::new(*box_compile(spec, context)?);
                    context.insert(name.clone(), spec_ptr.clone());
                    Ok(CompiledSpec::Name {
                        name,
                        spec: spec_ptr,
                    })
                }
            }
            Spec::Ref { name } => {
                let spec = context
                    .get(&name)
                    .ok_or_else(|| SpecCompileError::UndefinedName(name.clone()))?
                    .clone();
                Ok(CompiledSpec::Name { name, spec })
            }
            Spec::Record(raw_fields) => {
                let mut fields = Vec::with_capacity(raw_fields.len());
                let mut field_to_spec = HashMap::with_capacity(raw_fields.len());
                let mut field_to_index = HashMap::with_capacity(raw_fields.len());
                let mut duplicate_field_names = Vec::with_capacity(0);
                for (index, (field_name, field_spec)) in raw_fields.into_iter().enumerate() {
                    fields.push(field_name.clone());
                    match field_to_spec.insert(
                        field_name.clone(),
                        CompiledSpec::compile(field_spec, context)?,
                    ) {
                        Some(_) => duplicate_field_names.push(field_name.clone()),
                        None => {
                            field_to_index.insert(field_name.clone(), index);
                        }
                    }
                }
                if duplicate_field_names.is_empty() {
                    Ok(CompiledSpec::Record {
                        fields,
                        field_to_spec,
                        field_to_index,
                    })
                } else {
                    Err(SpecCompileError::DuplicateRecordFieldNames(
                        duplicate_field_names,
                    ))
                }
            }
            Spec::Tuple(fields) => {todo!()}
            Spec::Enum(variants) => todo!(),
            Spec::Union(variants) => todo!(),
            Spec::Void => Ok(CompiledSpec::Void),
        }
    }
}

impl TryFrom<Spec> for CompiledSpec {
    type Error = SpecCompileError;
    fn try_from(spec: Spec) -> Result<CompiledSpec, SpecCompileError> {
        CompiledSpec::compile(spec, &mut HashMap::new())
    }
}

#[inline]
fn box_compile(
    spec: Box<Spec>,
    context: &mut HashMap<String, Arc<CompiledSpec>>,
) -> Result<Box<CompiledSpec>, SpecCompileError> {
    Ok(Box::new(CompiledSpec::compile(*spec, context)?))
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct DecimalFmt {
    pub precision: u64,
    pub scale: u64,
}

pub struct IllegalDecimalFmt;

impl DecimalFmt {
    pub fn new(precision: u64, scale: u64) -> Result<DecimalFmt, IllegalDecimalFmt> {
        if scale <= precision {
            Ok(DecimalFmt { precision, scale })
        } else {
            Err(IllegalDecimalFmt)
        }
    }
}

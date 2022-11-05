use crate::spec::{
    InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size, Spec,
    StringEncodingFmt,
};
use std::{
    collections::{hash_set, HashMap, HashSet},
    hash::Hash,
    sync::Arc,
};
use strum::{EnumDiscriminants, EnumIter};

struct CompiledSchema {
    pub named_schema: HashMap<String, Arc<CompiledSchema>>,
    pub structure: CompiledSpecStructure,
}

#[derive(Debug, Eq, PartialEq, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
pub enum CompiledSpecStructure {
    Void,
    Bool,
    Uint(u8),
    Int(u8),
    BinaryFloatingPoint(InterchangeBinaryFloatingPointFormat),
    DecimalFloatingPoint(InterchangeDecimalFloatingPointFormat),
    Decimal(DecimalFmt),
    Map {
        size: Size,
        key_spec: Box<CompiledSpecStructure>,
        value_spec: Box<CompiledSpecStructure>,
    },
    List {
        size: Size,
        value_spec: Box<CompiledSpecStructure>,
    },
    String(Size, StringEncodingFmt),
    Bytes(Size),
    Optional(Box<CompiledSpecStructure>),
    Name(String),
    Record {
        fields: Vec<String>,
        field_to_spec: HashMap<String, CompiledSpecStructure>,
        field_to_index: HashMap<String, usize>,
    },
    Tuple(Vec<CompiledSpecStructure>),
    Enum {
        variants: Vec<String>,
        variant_to_spec: HashMap<String, CompiledSpecStructure>,
    },
    Union(Vec<CompiledSpecStructure>),
}

pub enum SpecCompileError {
    DuplicateName(String),
    UndefinedName(String),
    DuplicateRecordFieldNames(HashSet<String>),
    DuplicateRecordVariantNames(HashSet<String>),
    InfinitelyRecursiveType(HashSet<String>),
    IllegalDecimalFmt,
}

impl From<IllegalDecimalFmt> for SpecCompileError {
    fn from(_: IllegalDecimalFmt) -> Self {
        SpecCompileError::IllegalDecimalFmt
    }
}

impl CompiledSpecStructure {
    fn compile(
        spec: Spec,
        context: &mut HashMap<String, Arc<CompiledSpecStructure>>,
        non_optional_names: &mut HashSet<String>,
    ) -> Result<CompiledSpecStructure, SpecCompileError> {
        match spec {
            Spec::Bool => Ok(CompiledSpecStructure::Bool),
            Spec::Uint(n) => Ok(CompiledSpecStructure::Uint(n)),
            Spec::Int(n) => Ok(CompiledSpecStructure::Int(n)),
            Spec::BinaryFloatingPoint(fmt) => Ok(CompiledSpecStructure::BinaryFloatingPoint(fmt)),
            Spec::DecimalFloatingPoint(fmt) => Ok(CompiledSpecStructure::DecimalFloatingPoint(fmt)),
            Spec::Decimal { precision, scale } => Ok(CompiledSpecStructure::Decimal(
                DecimalFmt::new(precision, scale)?,
            )),
            Spec::Map {
                size,
                key_spec,
                value_spec,
            } => Ok(CompiledSpecStructure::Map {
                size,
                key_spec: box_compile(key_spec, context)?,
                value_spec: box_compile(value_spec, context)?,
            }),
            Spec::List { size, value_spec } => Ok(CompiledSpecStructure::List {
                size,
                value_spec: box_compile(value_spec, context)?,
            }),
            Spec::String(size, fmt) => Ok(CompiledSpecStructure::String(size, fmt)),
            Spec::Bytes(size) => Ok(CompiledSpecStructure::Bytes(size)),
            Spec::Optional(s) => Ok(CompiledSpecStructure::Optional(box_compile(s, context)?)),
            Spec::Name { name, spec } => {
                if context.contains_key(&name) {
                    Err(SpecCompileError::DuplicateName(name.clone()))
                } else {
                    context.insert(name.clone(), Arc::new(CompiledSpecStructure::Void));
                    non_optional_names.insert(name.clone());
                    let cs = Self::compile(*spec, context, non_optional_names)?;
                    context.insert(name.clone(), Arc::new(cs));
                    Ok(CompiledSpecStructure::Name(name))
                }
            }
            Spec::Ref { name } => {
                if non_optional_names.contains(&name) {
                    Err(SpecCompileError::InfinitelyRecursiveType(HashSet::from([
                        name,
                    ])))
                } else if context.contains_key(&name) {
                    Ok(CompiledSpecStructure::Name(name))
                } else {
                    Err(SpecCompileError::UndefinedName(name))
                }
            }
            Spec::Record(fields) => {
                let field_names = fields
                    .iter()
                    .map(|f| &f.0)
                    .map(|name| name.clone())
                    .collect();
                let mut duplicate_name_track = HashSet::new();
                let field_to_index = fields
                    .iter()
                    .enumerate()
                    .map(|(index, (name, _))| (name.clone(), index))
                    .collect();
                let mut field_to_spec = HashMap::with_capacity(fields.capacity());
                for (field_name, field_spec) in fields {
                    if field_to_spec
                        .insert(
                            field_name.clone(),
                            Self::compile(field_spec, context, &mut non_optional_names.clone())?,
                        )
                        .is_some()
                    {
                        duplicate_name_track.insert(field_name);
                    }
                }
                if duplicate_name_track.is_empty() {
                    Ok(CompiledSpecStructure::Record {
                        fields: field_names,
                        field_to_spec,
                        field_to_index,
                    })
                } else {
                    Err(SpecCompileError::DuplicateRecordFieldNames(
                        duplicate_name_track,
                    ))
                }
            }
            Spec::Tuple(fields) => {
                let mut compiled_fields = Vec::with_capacity(fields.capacity());
                for field_spec in fields {
                    compiled_fields.push(Self::compile(
                        field_spec,
                        context,
                        &mut non_optional_names.clone(),
                    )?)
                }
                Ok(CompiledSpecStructure::Tuple(compiled_fields))
            }
            Spec::Enum(variants) => {
                let variant_names: Vec<String> =
                    variants.iter().map(|v| &v.0).map(|n| n.clone()).collect();
                let mut all_names: HashSet<String> = variant_names.clone().into_iter().collect();
                let duplicate_names: HashSet<String> = variant_names
                    .iter()
                    .filter(|&n| all_names.remove(n))
                    .map(|n| n.clone())
                    .collect();
                if !duplicate_names.is_empty() {
                    return Err(SpecCompileError::DuplicateRecordVariantNames(
                        duplicate_names,
                    ));
                }
                let mut variant_to_spec = HashMap::new();
                compile_variants_with_loop_checking(
                    variants,
                    &mut variant_to_spec,
                    non_optional_names,
                    context,
                )?;
                Ok(CompiledSpecStructure::Enum {
                    variants: variant_names,
                    variant_to_spec,
                })
            }
            Spec::Union(variants) => {
                let len = variants.len();
                let variants: Vec<(usize, Spec)> = variants.into_iter().enumerate().collect();
                let mut variants_to_spec = HashMap::new();
                compile_variants_with_loop_checking(
                    variants,
                    &mut variants_to_spec,
                    non_optional_names,
                    context,
                )?;
                let compiled_variants = (0..len)
                    .map(|index| variants_to_spec.remove(&index).unwrap())
                    .collect();
                //Todo ENSURE UNIQUE SCHEMA with compiled schema variants
                Ok(CompiledSpecStructure::Union(compiled_variants))
            }
            Spec::Void => Ok(CompiledSpecStructure::Void),
        }
    }
}

// impl TryFrom<Spec> for CompiledSpec {
//     type Error = SpecCompileError;
//     fn try_from(spec: Spec) -> Result<CompiledSpec, SpecCompileError> {
//         CompiledSpec::compile(Resolve::reolvespec, &mut HashMap::new())
//     }
// }

#[inline]
fn box_compile(
    spec: Box<Spec>,
    context: &mut HashMap<String, Arc<CompiledSpecStructure>>,
) -> Result<Box<CompiledSpecStructure>, SpecCompileError> {
    Ok(Box::new(CompiledSpecStructure::compile(
        *spec,
        context,
        &mut HashSet::new(),
    )?))
}

#[inline]
fn compile_variants_with_loop_checking<T>(
    variants: Vec<(T, Spec)>,
    variant_to_spec: &mut HashMap<T, CompiledSpecStructure>,
    non_optional_names: &mut HashSet<String>,
    context: &mut HashMap<String, Arc<CompiledSpecStructure>>,
) -> Result<(), SpecCompileError>
where
    T: Eq + PartialEq + Hash + Clone,
{
    let num_variants = variants.len();
    let mut variants_with_non_optional_name_errors = HashSet::new();
    let mut offending_names_for_all_variants = HashSet::new();
    for (variant_name, variant_spec) in variants {
        let mut non_offending_names_for_variant = non_optional_names.clone();
        let mut offending_names_for_variant = HashSet::new();
        let cs = loop {
            //TODO get rid of cline somehow
            match CompiledSpecStructure::compile(
                variant_spec.clone(),
                context,
                &mut non_offending_names_for_variant,
            ) {
                Ok(cs) => break Ok(cs),
                Err(SpecCompileError::InfinitelyRecursiveType(offending_names)) => {
                    offending_names.iter().for_each(|offending_name| {
                        non_offending_names_for_variant.remove(offending_name);
                    });
                    offending_names.into_iter().for_each(|offending_name| {
                        offending_names_for_variant.insert(offending_name);
                    });
                    variants_with_non_optional_name_errors.insert(variant_name.clone());
                }
                Err(e) => break Err(e),
            }
        }?;
        offending_names_for_variant
            .into_iter()
            .for_each(|offending_name| {
                offending_names_for_all_variants.insert(offending_name);
            });
        variant_to_spec.insert(variant_name.clone(), cs);
    }
    if variants_with_non_optional_name_errors.len() == num_variants {
        Err(SpecCompileError::InfinitelyRecursiveType(
            offending_names_for_all_variants,
        ))
    } else {
        Ok(())
    }
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

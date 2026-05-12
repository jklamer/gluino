use crate::fingerprint::{SpecFingerprint, PLACE_HOLDER};
use crate::spec::{DecimalFmt, Spec, SpecCompileError, SpecType};
use crate::spec_parsing::{
    InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size,
    StringEncodingFmt,
};
use std::collections::{HashMap, HashSet};
use std::{iter, mem};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

trait SpecVisitor {
    fn visit_bool(&mut self);
    fn visit_uint(&mut self, n: u8);
    fn visit_int(&mut self, n: u8);
    fn visit_binary_fp(&mut self, fpf: &InterchangeBinaryFloatingPointFormat);
    fn visit_decimal_fp(&mut self, fp: &InterchangeDecimalFloatingPointFormat);
    fn visit_decimal(&mut self, precision: u64, scale: u64);
    fn visit_string(&mut self, size: Size, fmt: StringEncodingFmt);
    fn visit_bytes(&mut self, size: Size);
    fn visit_optional_start(&mut self);
    fn visit_optional_end(&mut self);
    fn visit_map_start_key(&mut self);
    fn visit_map_end_key(&mut self);
    fn visit_map_start_value(&mut self);
    fn visit_map_end_value(&mut self, size: Size);
    fn visit_list_start(&mut self);
    fn visit_list_end(&mut self, size: Size);
    fn visit_record_start(&mut self);
    fn visit_record_field_start(&mut self);
    fn visit_record_field_end(&mut self, field: String);
    fn visit_record_end(&mut self, fields: Vec<String>);
    fn visit_tuple_start(&mut self);
    fn visit_tuple_end(&mut self, n: u64);
    fn visit_enum_start(&mut self);
    fn visit_enum_variant_start(&mut self);
    fn visit_enum_variant_end(&mut self, field: String);
    fn visit_enum_end(&mut self, variants: Vec<String>);
    fn visit_union_start(&mut self);
    fn visit_union_member_start(&mut self);
    fn visit_union_member_end(&mut self);
    fn visit_union_end(&mut self);
    fn visit_const_set_start(&mut self);
    fn visit_const_set_end(&mut self, consts: Vec<Vec<u8>>);
    fn visit_name_start(&mut self, name: String);
    fn visit_name_end(&mut self, name: String);
    fn visit_ref(&mut self, name: String);
    fn visit_void(&mut self);
}

enum CompilerState {
    FAILED(Vec<SpecCompileError>),
    FUNCTIONAL,
}

impl CompilerState {
    fn fail(&mut self, compile_error: SpecCompileError) {
        match self {
            CompilerState::FAILED(errors) => {
                errors.push(compile_error);
            }
            CompilerState::FUNCTIONAL => {
                *self = CompilerState::FAILED(vec![compile_error])
            }
        };
    }
}

struct SpecCompiler {
    // Globally defined spec by name
    global_named_spec: HashMap<String, PreInitSpec>,
    state: CompilerState,
    spec_stack: Vec<StackSpec>,
    // Names whose necessary instantiation is required during each point in the visit
    name_in_direct_lineage: HashSet<String>,
    // names defined in the spec
    names_defined: HashSet<String>,
    //mapping of spec id to name
    // spec id is internal to Spec
    id_to_name: HashMap<u64, String>,
    // spec id generator
    name_counter: u64,
}

impl SpecCompiler {
    pub(crate) fn new() -> SpecCompiler {
        SpecCompiler {
            global_named_spec: Default::default(),
            state: CompilerState::FUNCTIONAL,
            spec_stack: vec![],
            name_in_direct_lineage: HashSet::new(),
            names_defined: Default::default(),
            id_to_name: Default::default(),
            name_counter: 0,
        }
    }

    fn finalize(mut self) -> Result<Spec, Vec<SpecCompileError>> {
        if let CompilerState::FAILED(errors) = self.state {
            Err(errors.into_iter().collect())
        } else if self.spec_stack.is_empty() {
            Err(vec![SpecCompileError::InternalCompilerError("No spec on compiler stack".into())])
        } else if let Some(stack_spec) = self.spec_stack.pop() && self.spec_stack.is_empty() {
            // TODO fingerprint and populate named specs
            todo!()
        } else {
            Err(vec![SpecCompileError::InternalCompilerError("Too many spec on compiler stack".into())])
        }
    }
}

type PreInitSpec = Spec;

struct StackSpec {
    //pre-fingerprinted Spec
    pub(crate) spec: PreInitSpec,
    // names required to be defined for this Spec
    pub(crate) names_required: HashSet<String>,
    // names of types that always get inited with this spec
    pub (crate) names_direct_children: HashSet<String>,
}

impl StackSpec {
    fn pre_init(spec_type: SpecType) -> StackSpec {
        StackSpec {
            spec: Spec {
                fingerprint: PLACE_HOLDER,
                named_spec: Default::default(),
                spec_type,
            },
            names_required: Default::default(),
            names_direct_children: Default::default(),
        }
    }

    fn names_required(self: Self, names_required: HashSet<String>) -> StackSpec {
        StackSpec {
            spec: self.spec,
            names_required,
            names_direct_children: self.names_direct_children,
        }
    }

    fn names_direct_children(self: Self, names_direct_children: HashSet<String>) -> StackSpec {
        StackSpec {
            spec: self.spec,
            names_required: self.names_required,
            names_direct_children,
        }
    }
}

impl SpecVisitor for SpecCompiler {
    fn visit_bool(&mut self) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::Bool))
    }

    fn visit_uint(&mut self, n: u8) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::Uint(n)))
    }

    fn visit_int(&mut self, n: u8) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::Int(n)))
    }

    fn visit_binary_fp(&mut self, fpf: &InterchangeBinaryFloatingPointFormat) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::BinaryFloatingPoint(fpf.clone())))
    }

    fn visit_decimal_fp(&mut self, fp: &InterchangeDecimalFloatingPointFormat) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::DecimalFloatingPoint(fp.clone())))
    }

    fn visit_decimal(&mut self, precision: u64, scale: u64) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::Decimal(DecimalFmt { precision, scale })))
    }

    fn visit_string(&mut self, size: Size, fmt: StringEncodingFmt) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::String(size, fmt)))
    }

    fn visit_bytes(&mut self, size: Size) {
        self.spec_stack.push(StackSpec::pre_init(SpecType::Bytes(size)))
    }

    fn visit_optional_start(&mut self) {
        self.name_in_direct_lineage.clear()
    }

    fn visit_optional_end(&mut self) {
        if let Some(contained_spec) = self.spec_stack.pop() {
            self.spec_stack.push(
                StackSpec::pre_init(SpecType::Optional(Box::new(contained_spec.spec))).names_required(contained_spec.names_required))
        } else if let CompilerState::FUNCTIONAL = self.state {
            // Internal compiler error
            self.state.fail(SpecCompileError::InternalCompilerError(
                "Optional end called with no contained spec".into(),
            ))
        }
    }

    fn visit_map_start_key(&mut self) {
        // no-op
        todo!(Only true if size does not contain 0);
        self.name_in_direct_lineage.clear()
    }

    fn visit_map_end_key(&mut self) {
        todo!()
    }

    fn visit_map_start_value(&mut self) {
        //no-op
        self.name_in_direct_lineage.clear()
    }

    fn visit_map_end_value(&mut self, size: Size) {
        if let (Some(value_stack_spec), Some(key_stack_spec)) = (self.spec_stack.pop(), self.spec_stack.pop()) {
            //todo!( just the maps here and build once)
            let stack_spec = StackSpec::pre_init(SpecType::Map {
                size: size.clone(),
                key_spec: Box::new(key_stack_spec.spec),
                value_spec: Box::new(value_stack_spec.spec),
            }).names_required(key_stack_spec.names_required.union(&value_stack_spec.names_required).cloned().collect());
            let stack_spec = if !size.can_be(0u64) {
                stack_spec.names_direct_children(key_stack_spec.names_direct_children.union(&value_stack_spec.names_direct_children).cloned().collect()))
            } else {
                stack_spec
            }
            self.spec_stack.push(stack_spec)
        } else if let CompilerState::FUNCTIONAL = self.state {
            // Internal compiler error
            self.state.fail(SpecCompileError::InternalCompilerError(
                "Map value end called with key and value not on stack".into(),
            ))
        }
    }

    fn visit_list_start(&mut self) {
        todo!(Only true if size does not contain 0);
        self.name_in_direct_lineage.clear()
    }

    fn visit_list_end(&mut self, size: Size) {
        if let Some(contained_spec) = self.spec_stack.pop() {
            let stack_spec = StackSpec::pre_init(SpecType::List {
                size: size.clone(),
                value_spec: Box::new(contained_spec.spec),
            }).names_required(contained_spec.names_required);
            let stack_spec = if !size.can_be(0u64) {
                stack_spec.names_direct_children(contained_spec.names_direct_children)
            } else {
                stack_spec
            };
            self.spec_stack.push(stack_spec)
        } else if let CompilerState::FUNCTIONAL = self.state {
            // Internal compiler error
            self.state.fail(SpecCompileError::InternalCompilerError(
                "List end called with no contained spec".into(),
            ))
        }
    }

    fn visit_record_start(&mut self) {
        // pass
    }

    fn visit_record_field_start(&mut self) {
        // pass
    }

    fn visit_record_field_end(&mut self, field: String) {
        // pass
    }

    fn visit_record_end(&mut self, fields: Vec<String>) {
        let mut field_to_spec = HashMap::new();
        let mut required_named_specs = HashSet::new();
        let mut agg_names_direct_children = HashSet::new();
        for field in fields.iter().rev() {
            if let Some(field_stack_spec) = self.spec_stack.pop() {
                required_named_specs.extend(field_stack_spec.names_required);
                agg_names_direct_children.extend(field_stack_spec.names_direct_children);
                if let Some(_) = field_to_spec.insert(field.clone(), field_stack_spec.spec) {
                    self.state.fail(SpecCompileError::DuplicateRecordFieldNames(HashSet::from_iter(iter::once(field.clone()))))
                }
            } else if let CompilerState::FUNCTIONAL = self.state {
                self.state.fail(SpecCompileError::InternalCompilerError(
                    "not enough record items on stack".into(),
                ))
            }
        }
        if let CompilerState::FUNCTIONAL = self.state {
            let field_to_index = fields
                .iter()
                .enumerate()
                .map(|(index, name)| (name.clone(), index))
                .collect();
            self.spec_stack.push(StackSpec::pre_init(SpecType::Record {
                fields,
                field_to_spec,
                field_to_index,
            }).names_required(required_named_specs)
                .names_direct_children(agg_names_direct_children));
        }
    }

    fn visit_tuple_start(&mut self) {
        //pass
    }

    fn visit_tuple_end(&mut self, n: u64) {
        let mut required_named_specs = HashSet::new();
        let mut agg_names_direct_children = HashSet::new();
        let mut items = vec![];
        for _ in 0..n {
            if let Some(item_stack_spec) = self.spec_stack.pop() {
                required_named_specs.extend(item_stack_spec.names_required);
                agg_names_direct_children.extend(item_stack_spec.names_direct_children);
                items.push(item_stack_spec.spec);
            } else if let CompilerState::FUNCTIONAL = self.state {
                self.state.fail(SpecCompileError::InternalCompilerError(
                    "not enough tuple items on stack".into(),
                ))
            }
        }
        // fix stack ordering
        items.reverse();
        self.spec_stack.push(StackSpec::pre_init(SpecType::Tuple(items))
            .names_required(required_named_specs)
            .names_direct_children(agg_names_direct_children));
    }

    fn visit_enum_start(&mut self) {
        todo!()
    }

    fn visit_enum_variant_start(&mut self) {
        todo!()
    }

    fn visit_enum_variant_end(&mut self, variant: String) {
        todo!()
    }

    //TODO test 3 branch composite infinite loop
    fn visit_enum_end(&mut self, variants: Vec<String>) {
        // 1) agg all required in this branch
        // 2) all names necessarily inited by this branch is intersection of all variant sets
        // 3) composite infinite loop check where the total number of variants with an infinite loop are all of em
        let mut required_named_specs = HashSet::new();
        let mut required_names_direct_children : Option<HashSet<String>> = None;
        let mut always_recursing_variant = 0usize;
        let mut recursing_names_agg = HashSet::new();
        let mut variants = vec![];
        for variant in variants.iter().rev() {
            if let Some(variant_stack_spec) = self.spec_stack.pop() {
                required_names_direct_children = if let Some(current_set) = required_names_direct_children {
                    Some(current_set.intersection(&variant_stack_spec.names_direct_children).into())
                } else {
                    Some(HashSet::from(&variant_stack_spec.names_direct_children))
                };
                // The variant is guaranteed to recurse
                if self.name_in_direct_lineage.intersection(&variant_stack_spec.names_direct_children).count() > 0 {
                    always_recursing_variant += 1;
                    recursing_names_agg.extend(self.name_in_direct_lineage.intersection(&variant_stack_spec.names_direct_children))
                }
                required_named_specs.extend(variant_stack_spec.names_required);
                variants.push((variant, variant_stack_spec.spec));
            } else if let CompilerState::FUNCTIONAL = self.state {
                self.state.fail(SpecCompileError::InternalCompilerError("not enought variant spec on stack".into()));
            }
        }
        if always_recursing_variant == variants.len() {
            self.state.fail(SpecCompileError::InfinitelyRecursiveTypes(recursing_names_agg.iter().cloned().collect()));
            return;
        }
        variants.reverse();
        self.spec_stack.push(StackSpec::pre_init(
            SpecType::Enum {
                variants: variants.iter().map(|v| v.0).collect(),
                variant_to_spec: variants.iter().collect(),
            }
        ))
    }

    fn visit_union_start(&mut self) {
        todo!()
    }

    fn visit_union_member_start(&mut self) {
        todo!()
    }

    fn visit_union_member_end(&mut self) {
        todo!()
    }

    fn visit_union_end(&mut self) {
        todo!()
    }

    fn visit_const_set_start(&mut self) {
        todo!()
    }

    fn visit_const_set_end(&mut self, consts: Vec<Vec<u8>>) {
        todo!()
    }

    fn visit_name_start(&mut self, name: String) {
        if self.names_defined.contains(&name) {
            self.state.fail(SpecCompileError::DuplicateName(name));
            return
        }
        let id = self.name_counter;
        self.name_counter += 1;
        self.names_defined.insert(name.clone());
        self.name_in_direct_lineage.insert(name.clone());
        self.id_to_name.insert(id, name);
    }

    fn visit_name_end(&mut self, name: String) {
        self.name_in_direct_lineage.remove(&name);
    }

    fn visit_ref(&mut self, name: String) {
        if self.name_in_direct_lineage.contains(&name) {
            self.state.fail(SpecCompileError::InfinitelyRecursiveTypes(HashSet::from([name])));
            return
        }
        if !self.names_defined.contains(&name) {
            self.state.fail(SpecCompileError::UndefinedName(name));
            return
        }
        self.spec_stack.push(StackSpec::pre_init(
            SpecType::Name(name.clone()))
            .names_required(HashSet::from([name.clone()]))
            .names_direct_children(HashSet::from([name.clone()])));
    }

    fn visit_void(&mut self) {
        todo!()
    }
}

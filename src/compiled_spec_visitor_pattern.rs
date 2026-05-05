use crate::fingerprint::SpecFingerprint;
use crate::spec::{DecimalFmt, Spec, SpecCompileError, SpecType};
use crate::spec_parsing::{
    InterchangeBinaryFloatingPointFormat, InterchangeDecimalFloatingPointFormat, Size,
    StringEncodingFmt,
};
use std::collections::{HashMap, HashSet};
use std::iter;

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
    fn visit_map_start_key(&mut self, size: Size);
    fn visit_map_end_key(&mut self);
    fn visit_map_start_value(&mut self);
    fn visit_map_end_value(&mut self);
    fn visit_list_start(&mut self, size: Size);
    fn visit_list_end(&mut self);
    fn visit_record_start(&mut self);
    fn visit_record_field_start(&mut self, field: String);
    fn visit_record_field_end(&mut self);
    fn visit_record_end(&mut self);
    fn visit_tuple_start(&mut self);
    fn visit_tuple_end(&mut self);
    fn visit_enum_start(&mut self);
    fn visit_enum_variant_start(&mut self, variant: String);
    fn visit_enum_variant_end(&mut self);
    fn visit_enum_end(&mut self);
    fn visit_union_start(&mut self);
    fn visit_union_member_start(&mut self);
    fn visit_union_member_end(&mut self);
    fn visit_union_end(&mut self);
    fn visit_const_set_start(&mut self);
    fn visit_const_set_end(&mut self, consts: Vec<Vec<u8>>);
    fn visit_name_start(&mut self, name: String);
    fn visit_name_end(&mut self);
    fn visit_ref(&mut self, name: String);
    fn visit_void(&mut self);
}

enum CompilerState {
    FAILED(HashSet<SpecCompileError>),
    FUNCTIONAL,
}

impl CompilerState {
    fn fail(&mut self, compile_error: SpecCompileError) {
        match self {
            CompilerState::FAILED(errors) => {
                errors.insert(compile_error);
            }
            CompilerState::FUNCTIONAL => {
                *self = CompilerState::FAILED(HashSet::from_iter(iter::once(compile_error)))
            }
        };
    }
}

struct SpecCompiler {
    global_named_spec: HashMap<String, Spec>,
    state: CompilerState,
    spec_stack: Vec<StackSpec>,
}

//pre-fingerprinted Spec
struct StackSpec {
    pub(crate) required_named_specs: HashSet<String>,
    pub(crate) spec_type: SpecType,
}

impl StackSpec {
    fn to_spec(
        self,
        globally_named_specs: &HashMap<String, Spec>,
    ) -> Result<Spec, Vec<SpecCompileError>> {
        let mut named_specs = HashMap::new();
        let mut missing_names = Vec::new();
        for name in self.required_named_specs {
            if let Some(named_spec) = globally_named_specs.get(&name) {
                named_specs.insert(name, named_spec.clone());
            } else {
                missing_names.push(SpecCompileError::UndefinedName(name));
            }
        }
        if !missing_names.is_empty() {
            Err(missing_names)
        } else {
            Ok(Spec {
                fingerprint: SpecFingerprint::new(&named_specs, &self.spec_type),
                named_spec: named_specs,
                spec_type: self.spec_type,
            })
        }
    }
}

impl SpecVisitor for SpecCompiler {
    fn visit_bool(&mut self) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::Bool,
        })
    }

    fn visit_uint(&mut self, n: u8) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::Uint(n),
        })
    }

    fn visit_int(&mut self, n: u8) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::Int(n),
        })
    }

    fn visit_binary_fp(&mut self, fpf: &InterchangeBinaryFloatingPointFormat) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::BinaryFloatingPoint(fpf.clone()),
        })
    }

    fn visit_decimal_fp(&mut self, fp: &InterchangeDecimalFloatingPointFormat) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::DecimalFloatingPoint(fp.clone()),
        })
    }

    fn visit_decimal(&mut self, precision: u64, scale: u64) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::Decimal(DecimalFmt { precision, scale }),
        })
    }

    fn visit_string(&mut self, size: Size, fmt: StringEncodingFmt) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::String(size, fmt),
        })
    }

    fn visit_bytes(&mut self, size: Size) {
        self.spec_stack.push(StackSpec {
            required_named_specs: Default::default(),
            spec_type: SpecType::Bytes(size),
        })
    }

    fn visit_optional_start(&mut self) {
        //noop
    }

    fn visit_optional_end(&mut self) {
        if let Some(contained_spec) = self.spec_stack.pop() {
            match contained_spec.to_spec(&self.global_named_spec) {
                Ok(compiled_contained_spec) => {
                    let required_named_specs =
                        compiled_contained_spec.named_spec.keys().cloned().collect();
                    let spec_type = SpecType::Optional(Box::new(compiled_contained_spec));
                    self.spec_stack.push(StackSpec {
                        required_named_specs,
                        spec_type,
                    })
                }
                Err(errs) => {
                    errs.into_iter().for_each(|err| self.state.fail(err));
                }
            }
        } else if let CompilerState::FUNCTIONAL = self.state {
            // Internal compiler error
            self.state.fail(SpecCompileError::InternalCompilerError(
                "Optional end called with no container spec".into(),
            ))
        }
    }

    fn visit_map_start_key(&mut self, size: Size) {
        todo!()
    }

    fn visit_map_end_key(&mut self) {
        todo!()
    }

    fn visit_map_start_value(&mut self) {
        todo!()
    }

    fn visit_map_end_value(&mut self) {
        todo!()
    }

    fn visit_list_start(&mut self, size: Size) {
        todo!()
    }

    fn visit_list_end(&mut self) {
        todo!()
    }

    fn visit_record_start(&mut self) {
        todo!()
    }

    fn visit_record_field_start(&mut self, field: String) {
        todo!()
    }

    fn visit_record_field_end(&mut self) {
        todo!()
    }

    fn visit_record_end(&mut self) {
        todo!()
    }

    fn visit_tuple_start(&mut self) {
        todo!()
    }

    fn visit_tuple_end(&mut self) {
        todo!()
    }

    fn visit_enum_start(&mut self) {
        todo!()
    }

    fn visit_enum_variant_start(&mut self, variant: String) {
        todo!()
    }

    fn visit_enum_variant_end(&mut self) {
        todo!()
    }

    fn visit_enum_end(&mut self) {
        todo!()
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
        todo!()
    }

    fn visit_name_end(&mut self) {
        todo!()
    }

    fn visit_ref(&mut self, name: String) {
        todo!()
    }

    fn visit_void(&mut self) {
        todo!()
    }
}

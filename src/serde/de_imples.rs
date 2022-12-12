use std::{io::Read, marker::PhantomData};

use gc::{Finalize, Trace};

use super::{encode::Encodable, GluinoValue, GluinoValueDe};

#[derive(Trace, Finalize)]
pub(crate) struct VoidGluinoValueDe;

impl<R> GluinoValueDe<R> for VoidGluinoValueDe
where
    R: Read,
{
    fn deserialize(&self, _: &mut R) -> Result<super::GluinoValue, super::GluinoDeserializationError> {
        Ok(GluinoValue::Void)
    }
}

pub(crate) struct NativeSingleDe<E: Encodable> {
    _d: PhantomData<E>,
}

impl <E:Encodable> NativeSingleDe<E> {
    pub(crate) fn new() -> NativeSingleDe<E> {
        NativeSingleDe { _d: PhantomData }
    }
}

impl<R: Read, E: Encodable> GluinoValueDe<R> for NativeSingleDe<E> {
    fn deserialize(&self, reader: &mut R) -> Result<GluinoValue, super::GluinoDeserializationError> {
        Ok(E::decode(reader)?)
    }
}

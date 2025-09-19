use std::{io::Read, marker::PhantomData};

use super::{encode::Encodable, GluinoDeserializationError, GluinoValue, GluinoValueDe};

pub(crate) struct VoidGluinoValueDe;

impl<R> GluinoValueDe<R> for VoidGluinoValueDe
where
    R: Read,
{
    fn deserialize(&self, _: &mut R) -> Result<GluinoValue, GluinoDeserializationError> {
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
    fn deserialize(&self, reader: &mut R) -> Result<GluinoValue, GluinoDeserializationError> {
        Ok(E::decode(reader)?)
    }
}

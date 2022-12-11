use std::io::Read;

use gc::{Trace, Finalize};

use super::{GluinoValueDe, GluinoValue};


#[derive(Trace, Finalize)]
pub(crate) struct VoidGluinoValueDe;

impl <R> GluinoValueDe<R> for VoidGluinoValueDe 
where 
    R: Read
{  
    fn deserialize(
        _: &mut R,
    ) -> Result<super::GluinoValue, super::GluinoDeserializationError> {
        Ok(GluinoValue::Void)
    }
}
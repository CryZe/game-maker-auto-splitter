use core::fmt;

use bstr::ByteSlice;

use crate::SmallStr;

#[allow(clippy::large_enum_variant)]
pub enum Variable {
    F64(f64),
    String(SmallStr),
    Undefined,
    Bool(bool),
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Variable::F64(v) => fmt::Debug::fmt(v, f),
            Variable::String(v) => fmt::Debug::fmt(v.as_bstr(), f),
            Variable::Undefined => f.write_str("undefined"),
            Variable::Bool(v) => fmt::Debug::fmt(v, f),
        }
    }
}

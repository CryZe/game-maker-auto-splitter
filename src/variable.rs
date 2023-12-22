use core::fmt;

use asr::Address64;
use bstr::ByteSlice;

use crate::SmallStr;

#[allow(clippy::large_enum_variant)]
pub enum Variable {
    /// 0
    Real(f64),
    /// 1
    String(SmallStr),
    /// 2
    Array(Address64),
    /// 3
    Ptr(Address64),
    /// 4
    Vec3,
    /// 5
    Undefined,
    /// 6
    Object(Address64),
    /// 7
    Int32(i32),
    /// 8
    Vec4,
    /// 9
    Matrix,
    /// 10
    Int64(i64),
    /// 11 / 0xb
    JsProperty,
    /// 13 / 0xd
    Bool(bool),
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Variable::Real(v) => fmt::Debug::fmt(v, f),
            Variable::String(v) => fmt::Debug::fmt(v.as_bstr(), f),
            Variable::Array(_) => f.write_str("Array"),
            Variable::Ptr(_) => f.write_str("Ptr"),
            Variable::Vec3 => f.write_str("Vec3"),
            Variable::Undefined => f.write_str("undefined"),
            Variable::Object(_) => f.write_str("Object"),
            Variable::Int32(v) => fmt::Debug::fmt(v, f),
            Variable::Vec4 => f.write_str("Vec4"),
            Variable::Matrix => f.write_str("Matrix"),
            Variable::Int64(v) => fmt::Debug::fmt(v, f),
            Variable::JsProperty => f.write_str("JsProperty"),
            Variable::Bool(v) => fmt::Debug::fmt(v, f),
        }
    }
}

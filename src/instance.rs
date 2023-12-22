use core::iter;

use asr::{Address64, Error, Process};
use bytemuck::{Pod, Zeroable};

use crate::{hash_map, variable::Variable, SmallStr};

/// CInstance
mod c_instance {
    /// CObjectGM* m_pObject
    pub const P_OBJECT_OFFSET: u64 = 0x90;
    /// int i_id
    pub const ID_OFFSET: u64 = 0xb4;
}

/// CObjectGM
mod c_object_gm {
    /// char* m_pName
    pub const P_NAME_OFFSET: u64 = 0x00;
}

/// YYObjectBase
mod yy_object_base {
    /// CHashMap<int,_RValue_*,_3>* m_yyvarsMap
    pub const YY_VARS_MAP_OFFSET: u64 = 0x48;
}

/// RValue
mod r_value {
    // /// union field_0
    // pub const FIELD_0_OFFSET: u64 = 0x0;
    // /// uint flags
    // pub const FLAGS_OFFSET: u64 = 0x8;
    /// uint kind
    pub const KIND_OFFSET: u64 = 0xc;
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
pub struct Instance {
    addr: Address64,
}

impl Instance {
    pub fn read_id(self, process: &Process) -> Result<i32, Error> {
        process.read::<i32>(self.addr + c_instance::ID_OFFSET)
    }

    pub fn read_object_name(self, process: &Process) -> Result<SmallStr, Error> {
        let obj_ptr = process.read::<Address64>(self.addr + c_instance::P_OBJECT_OFFSET)?;
        process.read(process.read::<Address64>(obj_ptr + c_object_gm::P_NAME_OFFSET)?)
    }

    pub fn read_variable(
        self,
        process: &Process,
        instance_var_lookup: Address64,
        name: &str,
    ) -> Result<(Address64, Option<Variable>), Error> {
        let slot = hash_map::lookup::<SmallStr, i32>(process, instance_var_lookup, name)?.unwrap();
        self.read_variable_by_slot(process, slot)
    }

    fn read_variable_by_slot(
        self,
        process: &Process,
        slot: i32,
    ) -> Result<(Address64, Option<Variable>), Error> {
        let yy_vars_map_ptr =
            process.read::<Address64>(self.addr + yy_object_base::YY_VARS_MAP_OFFSET)?;

        let Some(rv_ptr) = hash_map::lookup::<i32, Address64>(process, yy_vars_map_ptr, &slot)?
        else {
            return Ok((Address64::NULL, None));
        };

        if rv_ptr.is_null() {
            return Ok((rv_ptr, None));
        }

        let kind = process.read::<i32>(rv_ptr + r_value::KIND_OFFSET)? & 0x0ffffff;
        let variable = match kind {
            0 => Variable::Real(process.read(rv_ptr)?),
            1 => {
                let ref_thing = process.read::<Address64>(rv_ptr)?;
                let thing_ptr = process.read::<Address64>(ref_thing)?;
                let contents = process.read(thing_ptr)?;

                Variable::String(contents)
            }
            2 => Variable::Array(process.read(rv_ptr)?),
            3 => Variable::Ptr(process.read(rv_ptr)?),
            4 => Variable::Vec3,
            5 => Variable::Undefined,
            6 => Variable::Object(process.read(rv_ptr)?),
            7 => Variable::Int32(process.read(rv_ptr)?),
            8 => Variable::Vec4,
            9 => Variable::Matrix,
            10 => Variable::Int64(process.read(rv_ptr)?),
            11 => Variable::JsProperty,
            13 => Variable::Bool(process.read::<f64>(rv_ptr)? > 0.5),
            _ => return Ok((rv_ptr, None)),
        };
        Ok((rv_ptr, Some(variable)))
    }

    pub fn iter_variables(
        self,
        process: &Process,
        instance_var_lookup: Address64,
    ) -> Result<impl Iterator<Item = (SmallStr, Variable)> + '_, Error> {
        Ok(
            hash_map::iter::<SmallStr, i32>(process, instance_var_lookup)?.flat_map(
                move |(var_name, slot)| {
                    if let Ok((_, Some(var))) = self.read_variable_by_slot(process, slot) {
                        Some((var_name, var))
                    } else {
                        None
                    }
                },
            ),
        )
    }
}

pub fn iter_all(process: &Process, run_room: Address64) -> impl Iterator<Item = Instance> + '_ {
    const RUN_ROOM_LINKED_LIST_OFFSET: u64 = 216;
    const P_NEXT_PTR_OFFSET: u64 = 0x198;

    let instance = process
        .read::<Instance>(run_room + RUN_ROOM_LINKED_LIST_OFFSET)
        .ok();

    iter::successors(instance, move |&instance| {
        process
            .read::<Instance>(instance.addr + P_NEXT_PTR_OFFSET)
            .ok()
            .filter(|instance| !instance.addr.is_null())
    })
}

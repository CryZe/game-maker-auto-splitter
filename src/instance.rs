use core::iter;

use asr::{Address, Address64, Error, Process};
use bytemuck::{Pod, Zeroable};

use crate::{engine::Engine, hash_map, offset, variable::Variable, SmallStr};

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
pub struct Instance {
    addr: Address64,
}

impl Instance {
    pub fn read_id(self, process: &Process) -> Result<i32, Error> {
        process.read::<i32>(self.addr + offset::c_instance::ID)
    }

    pub fn read_object_name(self, process: &Process) -> Result<SmallStr, Error> {
        let obj_ptr = process.read::<Address64>(self.addr + offset::c_instance::P_OBJECT)?;
        process.read(process.read::<Address64>(obj_ptr + offset::c_object_gm::P_NAME)?)
    }

    pub fn read_variable(
        self,
        process: &Process,
        name: &str,
        engine: &Engine,
    ) -> Result<(Address64, Option<Variable>), Error> {
        let slot =
            hash_map::lookup::<SmallStr, i32>(process, engine.instance_var_lookup, name)?.unwrap();
        self.read_variable_by_slot(process, slot, engine)
    }

    fn read_variable_by_slot(
        self,
        process: &Process,
        slot: i32,
        engine: &Engine,
    ) -> Result<(Address64, Option<Variable>), Error> {
        let yy_vars_map_ptr =
            process.read::<Address64>(self.addr + offset::yy_object_base::YY_VARS_MAP)?;

        let Some(rv_ptr) = hash_map::lookup::<i32, Address64>(
            process,
            yy_vars_map_ptr.into(),
            &(
                slot,
                engine.version_specific.slot_to_var_map_uses_complex_hash,
            ),
        )?
        else {
            return Ok((Address64::NULL, None));
        };

        if rv_ptr.is_null() {
            return Ok((rv_ptr, None));
        }

        let kind = process.read::<i32>(rv_ptr + offset::r_value::KIND)? & 0x0ffffff;
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

    pub fn iter_variables<'both>(
        self,
        process: &'both Process,
        engine: &'both Engine,
    ) -> Result<impl Iterator<Item = (SmallStr, Variable)> + 'both, Error> {
        Ok(
            hash_map::iter::<SmallStr, i32>(process, engine.instance_var_lookup)?.flat_map(
                move |(var_name, slot)| {
                    if let Ok((_, Some(var))) = self.read_variable_by_slot(process, slot, engine) {
                        Some((var_name, var))
                    } else {
                        None
                    }
                },
            ),
        )
    }
}

pub fn iter_all<'both>(
    process: &'both Process,
    engine: &'both Engine,
) -> impl Iterator<Item = Instance> + 'both {
    let instance = process
        .read::<Instance>(
            engine.run_room + (offset::c_room::M_ACTIVE + offset::o_linked_list::M_P_FIRST),
        )
        .ok();

    iter::successors(instance, move |&instance| {
        process
            .read::<Instance>(
                Address::from(instance.addr) + engine.version_specific.c_instance_p_next_offset,
            )
            .ok()
            .filter(|instance| !instance.addr.is_null())
    })
}

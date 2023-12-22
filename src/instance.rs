use core::iter;

use asr::{Address64, Error, Process};
use bytemuck::{Pod, Zeroable};

use crate::{hash_map, variable::Variable, SmallStr};

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
pub struct Instance {
    addr: Address64,
}

impl Instance {
    pub fn read_id(self, process: &Process) -> Result<i32, Error> {
        const ID_OFFSET: u64 = 0xb4;
        process.read::<i32>(self.addr + ID_OFFSET)
    }

    pub fn read_object_name(self, process: &Process) -> Result<SmallStr, Error> {
        const P_OBJECT_OFFSET: u64 = 0x90;
        const NAME_OFFSET: u64 = 0x00;

        let obj_ptr = process.read::<Address64>(self.addr + P_OBJECT_OFFSET)?; // CObjectGM*
        process.read(process.read::<Address64>(obj_ptr + NAME_OFFSET)?)
    }

    pub fn read_variable(
        self,
        process: &Process,
        instance_var_lookup: Address64,
        name: &str,
    ) -> Result<(Address64, Option<Variable>), Error> {
        const YY_VARS_MAP_OFFSET: u64 = 0x48;
        const FLAGS_OFFSET: u64 = 8;
        const KIND_OFFSET: u64 = 12;

        let slot = hash_map::lookup::<SmallStr, i32>(process, instance_var_lookup, name)?.unwrap();
        let yy_vars_map = process.read::<Address64>(self.addr + YY_VARS_MAP_OFFSET)?;
        let rv_ptr = hash_map::lookup::<i32, Address64>(process, yy_vars_map, &slot)?.unwrap();
        if rv_ptr.is_null() {
            return Ok((rv_ptr, None));
        }

        let rkind = process.read::<i32>(rv_ptr + KIND_OFFSET)? & 0x0ffffff;
        let _rflags = process.read::<i32>(rv_ptr + FLAGS_OFFSET)?;

        let variable = match rkind {
            0 => Variable::F64(process.read(rv_ptr)?),
            1 => {
                let ref_thing = process.read::<Address64>(rv_ptr)?;
                let thing_ptr = process.read::<Address64>(ref_thing)?;
                let contents = process.read(thing_ptr)?;

                Variable::String(contents)
            }
            5 => Variable::Undefined,
            13 => Variable::Bool(process.read::<f64>(rv_ptr)? > 0.5),
            _ => unimplemented!(),
        };

        Ok((rv_ptr, Some(variable)))
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

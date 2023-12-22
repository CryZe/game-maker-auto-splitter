use core::iter;

use asr::{Address64, Error, Process};
use bytemuck::{Pod, Zeroable};

use crate::{get_var_by_slot, get_var_slot, variable::Variable, SmallStr};

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
        instancevarlookup: Address64,
        name: &str,
    ) -> Result<(Address64, Option<Variable>), Error> {
        let slot = get_var_slot(process, instancevarlookup, name);
        let yyvarsmapoffs = 0x48;
        let yyvarsmap = process.read::<Address64>(self.addr + yyvarsmapoffs)?;
        let rvptr = get_var_by_slot(process, yyvarsmap, slot);
        if rvptr.is_null() {
            return Ok((rvptr, None));
        }
        let flagsoffs = 8;
        let kindoffs = 12;
        let rkind = process.read::<i32>(rvptr + kindoffs)? & 0x0ffffff;
        let _rflags = process.read::<i32>(rvptr + flagsoffs)?;
        let variable = match rkind {
            0 => Variable::F64(process.read(rvptr)?),
            1 => {
                let refthing = process.read::<Address64>(rvptr)?;
                let thingptr = process.read::<Address64>(refthing)?;
                let contents = process.read(thingptr)?;

                Variable::String(contents)
            }
            5 => Variable::Undefined,
            13 => Variable::Bool(process.read::<f64>(rvptr)? > 0.5),
            _ => unimplemented!(),
        };

        Ok((rvptr, Some(variable)))
    }
}

pub fn iter_all(process: &Process, runroom: Address64) -> impl Iterator<Item = Instance> + '_ {
    const RUN_ROOM_LINKED_LIST_OFFSET: u64 = 216;
    const P_NEXT_PTR_OFFSET: u64 = 0x198;

    let instance = process
        .read::<Instance>(runroom + RUN_ROOM_LINKED_LIST_OFFSET)
        .ok();

    iter::successors(instance, move |&instance| {
        process
            .read::<Instance>(instance.addr + P_NEXT_PTR_OFFSET)
            .ok()
            .filter(|instance| !instance.addr.is_null())
    })
}

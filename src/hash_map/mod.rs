use core::mem;

use asr::{Address64, Error, Process};
use bytemuck::{Pod, Zeroable};

use crate::SmallStr;

mod murmurhash;

pub trait Hash: Sized {
    type SlotKey: Pod;
    type CompareKey: ?Sized;
    fn hash(compare_key: &Self::CompareKey) -> u32;
    fn read_from_slot(slot_key: &Self::SlotKey, process: &Process) -> Result<Self, Error>;
    fn matches(&self, compare_key: &Self::CompareKey) -> bool;
}

impl Hash for SmallStr {
    type SlotKey = Address64;
    type CompareKey = str;

    fn hash(compare_key: &Self::CompareKey) -> u32 {
        // GameMaker is using 0 as the seed.
        murmurhash::calculate(compare_key.as_bytes(), 0)
    }

    fn read_from_slot(slot_key: &Self::SlotKey, process: &Process) -> Result<Self, Error> {
        process.read(*slot_key)
    }

    fn matches(&self, compare_key: &Self::CompareKey) -> bool {
        self.matches(compare_key)
    }
}

impl Hash for i32 {
    type SlotKey = i32;
    type CompareKey = i32;

    fn hash(compare_key: &Self::CompareKey) -> u32 {
        (*compare_key as u32)
            .wrapping_mul(0x9E3779B1)
            .wrapping_add(1)
    }

    fn read_from_slot(slot_key: &Self::SlotKey, _process: &Process) -> Result<Self, Error> {
        Ok(*slot_key)
    }

    fn matches(&self, compare_key: &Self::CompareKey) -> bool {
        self == compare_key
    }
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct HashMap {
    size: u32,
    // TODO: This almost makes it seem like size and hashmask are rather u64, or
    // at least one of them.
    _unused: u32,
    mask: u32,
    _unused2: u32,
    elements: Address64,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Slot<K, V> {
    value: V,
    key: K,
    hash: u32,
}

unsafe impl<K: Pod, V: Pod> Pod for Slot<K, V> {}
unsafe impl<K: Zeroable, V: Zeroable> Zeroable for Slot<K, V> {}

pub fn lookup<K: Hash, V: Pod>(
    process: &Process,
    hash_map: Address64,
    key: &K::CompareKey,
) -> Result<Option<V>, Error> {
    let hash = K::hash(key) & 0x7fffffff;
    let hash_map = process.read::<HashMap>(hash_map)?; // m_curMask
    let mut ideal_pos = hash & hash_map.mask;

    let slot_size = mem::size_of::<Slot<K::SlotKey, V>>() as u64;

    for i in 0.. {
        let cur_slot = process
            .read::<Slot<K::SlotKey, V>>(hash_map.elements + ideal_pos as u64 * slot_size)?;

        if cur_slot.hash == 0 {
            break;
        }

        if cur_slot.hash == hash {
            let read_key = K::read_from_slot(&cur_slot.key, process)?;
            if read_key.matches(key) {
                return Ok(Some(cur_slot.value));
            }
        }

        // if ((int)((pMap->m_curSize + uIdealPos) - (curHash & uMask) & uMask) < iAddr)
        let slot_ideal_pos = cur_slot.hash & hash_map.mask;
        let difference = ideal_pos.wrapping_sub(slot_ideal_pos);
        let inbounds_difference = difference.wrapping_add(hash_map.size) & hash_map.mask;
        if inbounds_difference < i {
            break;
        }

        ideal_pos = ideal_pos.wrapping_add(1) & hash_map.mask;
    }

    Ok(None)
}

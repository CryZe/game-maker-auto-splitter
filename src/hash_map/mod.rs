use core::mem;

use asr::{Address, Address64, Error, Process};
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
    type CompareKey = (i32, bool);

    fn hash(&(compare_key, is_complex): &Self::CompareKey) -> u32 {
        (compare_key as u32)
            .wrapping_mul(is_complex as u32 * 0x9E3779B0 + 1)
            .wrapping_add(1)
    }

    fn read_from_slot(slot_key: &Self::SlotKey, _process: &Process) -> Result<Self, Error> {
        Ok(*slot_key)
    }

    fn matches(&self, (compare_key, _): &Self::CompareKey) -> bool {
        self == compare_key
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct CHashMap {
    cur_size: u32,
    _num_used: u32,
    cur_mask: u32,
    _grow_threshold: u32,
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

/// Based on Code_Variable_Find_Slot_From_Name
pub fn lookup<K: Hash, V: Pod>(
    process: &Process,
    hash_map: Address,
    key: &K::CompareKey,
) -> Result<Option<V>, Error> {
    let hash = K::hash(key) & 0x7fffffff;
    let hash_map = process.read::<CHashMap>(hash_map)?;
    let mut ideal_pos = hash & hash_map.cur_mask;

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
        let slot_ideal_pos = cur_slot.hash & hash_map.cur_mask;
        let difference = ideal_pos.wrapping_sub(slot_ideal_pos);
        let inbounds_difference = difference.wrapping_add(hash_map.cur_size) & hash_map.cur_mask;
        if inbounds_difference < i {
            break;
        }

        ideal_pos = ideal_pos.wrapping_add(1) & hash_map.cur_mask;
    }

    Ok(None)
}

pub fn iter<K: Hash + 'static, V: Pod>(
    process: &Process,
    hash_map: Address,
) -> Result<impl Iterator<Item = (K, V)> + '_, Error> {
    let hash_map = process.read::<CHashMap>(hash_map)?;

    Ok((0..hash_map.cur_size as u64).flat_map(move |i| {
        let slot_size = mem::size_of::<Slot<K::SlotKey, V>>() as u64;

        let cur_slot = process
            .read::<Slot<K::SlotKey, V>>(hash_map.elements + i * slot_size)
            .ok()?;

        if cur_slot.hash == 0 {
            return None;
        }

        let read_key = K::read_from_slot(&cur_slot.key, process).ok()?;
        Some((read_key, cur_slot.value))
    }))
}

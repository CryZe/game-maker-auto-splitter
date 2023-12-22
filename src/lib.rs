#![no_std]

use asr::{future::next_tick, signature::Signature, string::ArrayCString, Address64, Process};
use bstr::ByteSlice;

asr::async_main!(stable);
asr::panic_handler!(print: always);

static VARLOOKUPSIG: Signature<38> = Signature::new("48 83 ec 28 48 8b 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 48 85 c0 74 07 8b 00 48 83 c4 28 c3 b8 ff ff ff ff 48 83 c4 28 c3");
static RUNROOMSIG: Signature<35> = Signature::new("48 b8 00 00 00 00 00 00 10 c0 41 c7 40 0c 00 00 00 00 49 89 00 48 8b 05 ?? ?? ?? ?? 48 85 c0 74 48 85 d2");
static GPGLOBALSIG: Signature<23> =
    Signature::new("e8 ?? ?? ?? ?? 48 8b 3d ?? ?? ?? ?? 33 ed 48 8b c8 48 89 2b 89 6b 08");

type SmallStr = ArrayCString<0xFF>;

mod instance;
mod murmurhash;
mod variable;

fn hash_var_name(n: &str) -> u32 {
    // GameMaker is using 0 as the seed.
    murmurhash::calculate(n.as_bytes(), 0)
}

fn hash_var_slot(varslot: i32) -> u32 {
    (varslot as u32).wrapping_mul(0x9E3779B1).wrapping_add(1)
}

fn get_var_slot(process: &Process, instancevarlookup: Address64, name: &str) -> i32 {
    let hash = hash_var_name(name);
    let hashmask = process.read::<u32>(instancevarlookup + 8).unwrap(); // m_curMask
    let mut idealpos = (hashmask & hash & 0x7fffffff) as i32;
    let elements = process.read::<Address64>(instancevarlookup + 16).unwrap(); // m_pElements
    let offhash = 16; // .h
    let offk = 8; // .k
    let offv = 0; // .v
    let elsize = 24; // sizeof(Element)
    let cursize = process.read::<u32>(instancevarlookup + 0).unwrap(); // m_numUsed

    let mut curhash = process
        .read::<u32>(elements + (idealpos * elsize) + offhash)
        .unwrap();
    if curhash != 0 {
        let mut i = -1;
        loop {
            if curhash == (hash & 0x7fffffff) {
                let key = process
                    .read::<SmallStr>(
                        process
                            .read::<Address64>(elements + (idealpos * elsize) + offk)
                            .unwrap(),
                    )
                    .unwrap();
                if key.matches(name) {
                    return process
                        .read::<i32>(elements + (idealpos * elsize) + offv)
                        .unwrap();
                }
            }
            i += 1;
            //if ((int)((pMap->m_curSize + uIdealPos) - (curHash & uMask) & uMask) < iAddr)
            if (((cursize.wrapping_add_signed(idealpos) - (curhash & hashmask)) & hashmask) as i32)
                < i
            {
                return -1;
            }
            idealpos = (idealpos + 1) & (hashmask as i32);
            curhash = process
                .read::<u32>(elements + (idealpos * elsize) + offhash)
                .unwrap();
            if curhash == 0 {
                break;
            }
        }
    }
    -1
}

fn get_var_by_slot(process: &Process, yyvars: Address64, slot: i32) -> Address64 {
    let hash = hash_var_slot(slot);
    let hashmask = process.read::<u32>(yyvars + 8).unwrap(); // m_curMask
    let mut idealpos = (hashmask & hash & 0x7fffffff) as i32;
    let elements = process.read::<Address64>(yyvars + 16).unwrap(); // m_pElements
    let offhash = 12; // .h
    let offk = 8; // .k
    let offv = 0; // .v
    let elsize = 16; // sizeof(Element)
    let cursize = process.read::<u32>(yyvars + 0).unwrap(); // m_numUsed

    let mut curhash = process
        .read::<u32>(elements + (idealpos * elsize) + offhash)
        .unwrap();
    if curhash != 0 {
        let mut i = -1;
        loop {
            if curhash == (hash & 0x7fffffff) {
                let key = process
                    .read::<i32>(elements + (idealpos * elsize) + offk)
                    .unwrap();
                if key == slot {
                    return process
                        .read::<Address64>(elements + (idealpos * elsize) + offv)
                        .unwrap();
                }
            }
            i += 1;
            if (((cursize.wrapping_add_signed(idealpos) - (curhash & hashmask)) & hashmask) as i32)
                < i
            {
                return Address64::NULL;
            }
            idealpos = (idealpos + 1) & (hashmask as i32);
            curhash = process
                .read::<u32>(elements + (idealpos * elsize) + offhash)
                .unwrap();
            if curhash == 0 {
                break;
            }
        }
    }
    Address64::NULL
}

async fn main() {
    asr::print_message("Hello, World!");

    let process_name = "Mondealy.exe";

    loop {
        let process = Process::wait_attach(process_name).await;
        process
            .until_closes(async {
                let module_range = process.get_module_range(process_name).unwrap();

                let varlookupsig = VARLOOKUPSIG
                    .scan_process_range(&process, module_range)
                    .unwrap();

                let runroomsig = RUNROOMSIG
                    .scan_process_range(&process, module_range)
                    .unwrap();

                let gpglobalsig = GPGLOBALSIG
                    .scan_process_range(&process, module_range)
                    .unwrap();

                let instancevarlookupptr =
                    varlookupsig + 11 + process.read::<u32>(varlookupsig + 7).unwrap();
                let instancevarlookup = process.read::<Address64>(instancevarlookupptr).unwrap();
                //let instancevarlookup = instancevarlookupptr;

                let runroomptr = runroomsig + 28 + process.read::<u32>(runroomsig + 24).unwrap();
                let runroom = process.read::<Address64>(runroomptr).unwrap();

                let gpglobalptr = gpglobalsig + 12 + process.read::<u32>(gpglobalsig + 8).unwrap();
                let _gpglobal = process.read::<Address64>(gpglobalptr).unwrap();

                for (index, instance) in instance::iter_all(&process, runroom).enumerate() {
                    let instance_id = instance.read_id(&process).unwrap();
                    let object_name = instance.read_object_name(&process).unwrap();

                    asr::print_limited::<256>(&format_args!(
                        "instance {index} = {instance_id} ({})",
                        object_name.as_bstr()
                    ));

                    if object_name.matches("obj_male") {
                        asr::print_limited::<256>(&format_args!(
                            "obj_male.anim_suffix = {:?}",
                            instance
                                .read_variable(&process, instancevarlookup, "anim_suffix")
                                .unwrap()
                                .1
                        ));
                    }
                }

                loop {
                    next_tick().await;
                }
            })
            .await;
    }
}

#![no_std]

use asr::{
    future::next_tick,
    signature::Signature,
    string::{ArrayCString, ArrayString},
    Address64, Process,
};
use bstr::ByteSlice;

asr::async_main!(stable);
asr::panic_handler!(print: always);

static VAR_LOOKUP_SIG: Signature<38> = Signature::new("48 83 ec 28 48 8b 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 48 85 c0 74 07 8b 00 48 83 c4 28 c3 b8 ff ff ff ff 48 83 c4 28 c3");
static RUN_ROOM_SIG: Signature<35> = Signature::new("48 b8 00 00 00 00 00 00 10 c0 41 c7 40 0c 00 00 00 00 49 89 00 48 8b 05 ?? ?? ?? ?? 48 85 c0 74 48 85 d2");
static GP_GLOBAL_SIG: Signature<23> =
    Signature::new("e8 ?? ?? ?? ?? 48 8b 3d ?? ?? ?? ?? 33 ed 48 8b c8 48 89 2b 89 6b 08");

type SmallStr = ArrayCString<0xFF>;

mod hash_map;
mod instance;
mod variable;

async fn main() {
    asr::print_message("Hello, World!");

    let process_name = "Mondealy.exe";

    loop {
        let process = Process::wait_attach(process_name).await;
        process
            .until_closes(async {
                let module_range = process.get_module_range(process_name).unwrap();

                let var_lookup_sig = VAR_LOOKUP_SIG
                    .scan_process_range(&process, module_range)
                    .unwrap();

                let run_room_sig = RUN_ROOM_SIG
                    .scan_process_range(&process, module_range)
                    .unwrap();

                let gp_global_sig = GP_GLOBAL_SIG
                    .scan_process_range(&process, module_range)
                    .unwrap();

                let instance_var_lookup_ptr =
                    var_lookup_sig + 11 + process.read::<u32>(var_lookup_sig + 7).unwrap();
                let instance_var_lookup =
                    process.read::<Address64>(instance_var_lookup_ptr).unwrap();
                //let instancevarlookup = instancevarlookupptr;

                let run_room_ptr =
                    run_room_sig + 28 + process.read::<u32>(run_room_sig + 24).unwrap();
                let run_room = process.read::<Address64>(run_room_ptr).unwrap();

                let gp_global_ptr =
                    gp_global_sig + 12 + process.read::<u32>(gp_global_sig + 8).unwrap();
                let _gp_global = process.read::<Address64>(gp_global_ptr).unwrap();

                let obj_male = instance::iter_all(&process, run_room)
                    .find(|instance| {
                        let object_name = instance.read_object_name(&process).unwrap();
                        object_name.matches("obj_male")
                    })
                    .unwrap();

                let mut buf = ArrayString::<256>::new();

                loop {
                    for (var_name, value) in obj_male
                        .iter_variables(&process, instance_var_lookup)
                        .unwrap()
                    {
                        use core::fmt::Write;
                        buf.clear();
                        let _ = write!(buf, "{}", var_name.as_bstr());
                        let mid = buf.len();
                        let _ = write!(buf, "{value:?}");
                        let (key, value) = buf.split_at(mid);
                        asr::timer::set_variable(key, value);
                    }
                    next_tick().await;
                }
            })
            .await;
    }
}

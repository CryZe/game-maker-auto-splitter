#![no_std]

use asr::{
    future::next_tick,
    string::{ArrayCString, ArrayString},
    Process,
};
use bstr::ByteSlice;
use engine::Engine;

asr::async_main!(stable);
asr::panic_handler!(print: always);

type SmallStr = ArrayCString<0xFF>;

mod engine;
mod hash_map;
mod instance;
mod offset;
mod variable;

async fn main() {
    asr::print_message("Hello, World!");

    let process_name = "Mondealy.exe";
    // let process_name = "Space Rocks.exe";

    loop {
        let process = Process::wait_attach(process_name).await;
        process
            .until_closes(async {
                let engine = Engine::attach(&process, process_name).await;

                let instance = instance::iter_all(&process, &engine)
                    .find(|instance| {
                        let object_name = instance.read_object_name(&process).unwrap_or_default();
                        asr::print_limited::<256>(&format_args!("{}", object_name.as_bstr()));
                        // object_name.matches("Object1")
                        object_name.matches("obj_core")
                    })
                    .unwrap();

                let mut buf = ArrayString::<256>::new();

                for (var_name, value) in instance.iter_variables(&process, &engine).unwrap() {
                    use core::fmt::Write;
                    buf.clear();
                    let _ = write!(buf, "{}", var_name.as_bstr());
                    let mid = buf.len();
                    let _ = write!(buf, "{value:?}");
                    let (key, value) = buf.split_at(mid);
                    asr::timer::set_variable(key, value);
                }

                loop {
                    next_tick().await;
                }
            })
            .await;
    }
}

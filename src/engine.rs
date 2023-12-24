use asr::{future::retry, signature::Signature, Address, Address64, Process};

/// sub rsp,28
/// mov rcx,??
/// call ??
/// test rax,rax
/// je +7
/// mov eax,[rax]
/// add rsp,28
/// ret
/// mov eax,FFFFFFFF
/// add rsp,28
/// ret
const MOV_INST_MONDEALY_OFFSET: u64 = 7;
static VAR_LOOKUP_SIG_MONDEALY: Signature<38> = Signature::new(
    "
    48 83 ec 28
    48 8b 0d ????????
    e8 ????????
    48 85 c0
    74 07
    8b 00
    48 83 c4 28
    c3
    b8 ffffffff
    48 83 c4 28
    c3
    ",
);

// mov [rsp+08],rbx
// mov [rsp+10],rbp
// mov [rsp+18],rsi
// push rdi
// push r14
// push r15
// sub rsp,20
// mov r14,?
// mov rcx,rdx
// mov r15,rdx
// call ?
// mov r11d,[r14+08]
// mov ebp,eax
// mov rsi,[r14+10]
// btr ebp,1F
const MOV_INST_NOV_2023_OFFSET: u64 = 27;
static VAR_LOOKUP_SIG_NOV_2023: Signature<56> = Signature::new(
    "
    48 89 5C 24 08
    48 89 6C 24 10
    48 89 74 24 18
    57
    41 56
    41 57
    48 83 EC 20
    4C 8B 35 ????????
    48 8B CA
    4C 8B FA
    E8 ????????
    45 8B 5E 08
    8B E8
    49 8B 76 10
    0FBA F5 1F
    ",
);

static RUN_ROOM_SIG: Signature<35> = Signature::new("48 b8 00 00 00 00 00 00 10 c0 41 c7 40 0c 00 00 00 00 49 89 00 48 8b 05 ?? ?? ?? ?? 48 85 c0 74 48 85 d2");
// static GP_GLOBAL_SIG: Signature<23> =
//     Signature::new("e8 ?? ?? ?? ?? 48 8b 3d ?? ?? ?? ?? 33 ed 48 8b c8 48 89 2b 89 6b 08");

pub struct Engine {
    pub instance_var_lookup: Address,
    pub run_room: Address,
    pub version_specific: &'static VersionSpecific,
}

pub struct VersionSpecific {
    pub slot_to_var_map_uses_complex_hash: bool,
    /// On CInstance: CInstance* m_pNext
    pub c_instance_p_next_offset: u16,
}

impl Engine {
    pub async fn attach(process: &Process, process_name: &str) -> Self {
        let module_range = process.wait_module_range(process_name).await;

        // let gp_global_sig = GP_GLOBAL_SIG
        //     .scan_process_range(&process, module_range)
        //     .unwrap();

        // let var_lookup_sig = VAR_LOOKUP_SIG_MONDEALY
        //     .scan_process_range(&process, module_range)
        //     .unwrap();

        // let instance_var_lookup_ptr = var_lookup_sig
        //     + (MOV_RCX_MONDEALY_OFFSET + 4)
        //     + process
        //         .read::<u32>(var_lookup_sig + MOV_RCX_MONDEALY_OFFSET)
        //         .unwrap();
        // let instance_var_lookup =
        //     process.read::<Address64>(instance_var_lookup_ptr).unwrap();

        let (var_lookup_sig, version_specific) = retry(|| {
            if let Some(addr) = VAR_LOOKUP_SIG_NOV_2023.scan_process_range(process, module_range) {
                return Some((
                    addr + MOV_INST_NOV_2023_OFFSET,
                    &VersionSpecific {
                        slot_to_var_map_uses_complex_hash: false,
                        c_instance_p_next_offset: 0x1A0,
                    },
                ));
            }
            if let Some(addr) = VAR_LOOKUP_SIG_MONDEALY.scan_process_range(process, module_range) {
                return Some((
                    addr + MOV_INST_MONDEALY_OFFSET,
                    &VersionSpecific {
                        slot_to_var_map_uses_complex_hash: true,
                        c_instance_p_next_offset: 0x198,
                    },
                ));
            }
            None
        })
        .await;

        let instance_var_lookup = retry(|| {
            let instance_var_lookup_ptr =
                var_lookup_sig + 4 + process.read::<u32>(var_lookup_sig)?;

            process.read::<Address64>(instance_var_lookup_ptr)
        })
        .await;

        let run_room_sig = retry(|| RUN_ROOM_SIG.scan_process_range(process, module_range)).await;

        let run_room = retry(|| {
            let run_room_ptr = run_room_sig + 28 + process.read::<u32>(run_room_sig + 24)?;
            process.read::<Address64>(run_room_ptr)
        })
        .await;

        // let gp_global_ptr =
        //     gp_global_sig + 12 + process.read::<u32>(gp_global_sig + 8).unwrap();
        // let _gp_global = process.read::<Address64>(gp_global_ptr).unwrap();

        Self {
            instance_var_lookup: instance_var_lookup.into(),
            run_room: run_room.into(),
            version_specific,
        }
    }
}

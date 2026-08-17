// Based on https://wiki.osdev.org/Porting_Rust_standard_library
use crate::spec::{Cc, LinkerFlavor, Lld, Os, StackProbeType, TargetOptions, CodeModel};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: Os::Other("d3os".into()),
        linker: Some("rust-lld".into()),
        linker_flavor: LinkerFlavor::Gnu(Cc::No, Lld::Yes),
        stack_probes: StackProbeType::Inline,
        code_model: Some(CodeModel::Large),
        has_thread_local: false,
        ..Default::default()
    }
}
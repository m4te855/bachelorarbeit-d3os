//use crate::spec::crt_objects;
use crate::spec::{Cc, LinkerFlavor, Lld, StackProbeType, TargetOptions, CodeModel};

pub(crate) fn opts() -> TargetOptions {
    TargetOptions {
        os: "d3os".into(),
        linker: Some("rust-lld".into()),
        linker_flavor: LinkerFlavor::Gnu(Cc::No, Lld::Yes),
        stack_probes: StackProbeType::Inline,
        code_model: Some(CodeModel::Large),
        //relocation_model: RelocModel::Static,
        // Following two only if using external CRT to provide _start
        //pre_link_objects: crt_objects::pre_d3os(),
        //post_link_objects: crt_objects::post_d3os(),
        has_thread_local: false, //<---!!!
        ..Default::default()
    }
}
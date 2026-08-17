// Based on https://wiki.osdev.org/Porting_Rust_standard_library
use crate::spec::{base, Arch, PanicStrategy, Target, TargetMetadata};

pub(crate) fn target() -> Target {
    let mut base = base::d3os::opts();
    base.cpu = "x86-64".into();
    base.disable_redzone = true;
    base.panic_strategy = PanicStrategy::Abort;
    base.features = "-mmx,-sse,+soft-float".into();

    Target {
        llvm_target: "x86_64-unknown-linux-gnu".into(),
        pointer_width: 64,
        data_layout:
            "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128".into(),
        arch: Arch::X86_64,
        options: base,
        metadata: TargetMetadata {
            description: None,
            tier: None,
            host_tools: None,
            std: None,
        },
    }
}
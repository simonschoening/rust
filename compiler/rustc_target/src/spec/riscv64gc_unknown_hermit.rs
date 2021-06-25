use crate::spec::{CodeModel, RelocModel, TlsModel};
use crate::spec::Target;

pub fn target() -> Target {
    let mut base = super::hermit_base::opts();
    base.cpu = "generic-rv64".to_string();
    base.max_atomic_width = Some(64);
    base.features = "+m,+a,+f,+d,+c".to_string();
    base.unsupported_abis = super::riscv_base::unsupported_abis();
    base.code_model = Some(CodeModel::Medium);
    base.relocation_model = RelocModel::Pic;
    base.tls_model = TlsModel::LocalExec;
    base.llvm_abiname = "lp64d".to_string();

    Target {
        llvm_target: "riscv64-unknown-hermit".to_string(),
        pointer_width: 64,
        data_layout: "e-m:e-p:64:64-i64:64-i128:128-n64-S128".to_string(),
        arch: "riscv64".to_string(),
        options: base,
    }
}

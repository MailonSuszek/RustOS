// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Compartilha o estado de cada processador auxiliar iniciado pelo nucleo.
use core::sync::atomic::AtomicU32;

pub const AP_PENDENTE: u32 = 0;
pub const AP_INICIALIZADO: u32 = 1;
pub const AP_FALHOU: u32 = 2;

#[repr(C)]
pub struct ApInicializacao {
	pub apic_id: AtomicU32,
	pub nmi_lint_mask: AtomicU32,
	pub nmi_lint0_flags: AtomicU32,
	pub nmi_lint1_flags: AtomicU32,
	pub estado: AtomicU32,
}

impl ApInicializacao {
	pub const fn novo() -> Self {
		Self {
			apic_id: AtomicU32::new(0),
			nmi_lint_mask: AtomicU32::new(0),
			nmi_lint0_flags: AtomicU32::new(0),
			nmi_lint1_flags: AtomicU32::new(0),
			estado: AtomicU32::new(AP_PENDENTE),
		}
	}
}

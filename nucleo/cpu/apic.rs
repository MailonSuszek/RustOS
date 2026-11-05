// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Inicializa e configura o controlador local APIC.
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::virtual_memory;

const APIC_BASE_MSR: u32 = 0x1b;
const APIC_BASE_ADDRESS_MASK: u64 = 0x0000_000f_ffff_f000;
const APIC_GLOBAL_ENABLE: u64 = 1 << 11;
const X2APIC_ENABLE: u64 = 1 << 10;
const APIC_SPURIOUS_VECTOR: u32 = 0x1ff;
const APIC_SPURIOUS_VECTOR_OFFSET: usize = 0xf0 / core::mem::size_of::<u32>();
const X2APIC_SPURIOUS_VECTOR_MSR: u32 = 0x80f;
const X2APIC_ID_MSR: u32 = 0x802;
const X2APIC_ICR_MSR: u32 = 0x830;
const X2APIC_LVT_LINT0_MSR: u32 = 0x835;
const X2APIC_LVT_LINT1_MSR: u32 = 0x836;
const XAPIC_ID_OFFSET: usize = 0x20 / core::mem::size_of::<u32>();
const XAPIC_ICR_LOW_OFFSET: usize = 0x300 / core::mem::size_of::<u32>();
const XAPIC_ICR_HIGH_OFFSET: usize = 0x310 / core::mem::size_of::<u32>();
const XAPIC_LVT_LINT0_OFFSET: usize = 0x350 / core::mem::size_of::<u32>();
const XAPIC_LVT_LINT1_OFFSET: usize = 0x360 / core::mem::size_of::<u32>();
const ICR_DELIVERY_PENDING: u32 = 1 << 12;
const LVT_DELIVERY_MODE_MASK: u32 = 7 << 8;
const LVT_DELIVERY_MODE_NMI: u32 = 4 << 8;
const LVT_POLARITY_ACTIVE_LOW: u32 = 1 << 13;
const LVT_TRIGGER_LEVEL: u32 = 1 << 15;
const LVT_MASKED: u32 = 1 << 16;
const MAX_IPI_POLLS: usize = 1_000_000;
static X2APIC_MODE: AtomicBool = AtomicBool::new(false);
static APIC_PHYSICAL_BASE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroApic {
	CpuSemApic,
	EnderecoInvalido,
	MapeamentoVirtual(virtual_memory::ErroVirtual),
	DestinoInvalido,
	IpiNaoConcluido,
	FlagsNmiInvalidas,
	LintNmiInvalido,
}

pub(super) fn inicializar() -> Result<(), ErroApic> {
	let features = core::arch::x86_64::__cpuid(1);
	if features.edx & (1 << 9) == 0 {
		return Err(ErroApic::CpuSemApic);
	}

	let mut apic_base = unsafe { read_msr(APIC_BASE_MSR) };
	if apic_base & X2APIC_ENABLE != 0 {
		if features.ecx & (1 << 21) == 0 {
			return Err(ErroApic::CpuSemApic);
		}
		apic_base |= APIC_GLOBAL_ENABLE;
		X2APIC_MODE.store(true, Ordering::Release);
		unsafe {
			write_msr(APIC_BASE_MSR, apic_base);
			write_msr(X2APIC_SPURIOUS_VECTOR_MSR, u64::from(APIC_SPURIOUS_VECTOR));
		}
		return Ok(());
	}

	let physical_base = apic_base & APIC_BASE_ADDRESS_MASK;
	if physical_base == 0 {
		return Err(ErroApic::EnderecoInvalido);
	}
	virtual_memory::mapear_dispositivo(physical_base, 4096)
		.map_err(ErroApic::MapeamentoVirtual)?;

	apic_base |= APIC_GLOBAL_ENABLE;
	APIC_PHYSICAL_BASE.store(physical_base, Ordering::Release);
	X2APIC_MODE.store(false, Ordering::Release);
	unsafe {
		write_msr(APIC_BASE_MSR, apic_base);
		let spurious_vector = (physical_base as *mut u32).add(APIC_SPURIOUS_VECTOR_OFFSET);
		ptr::write_volatile(spurious_vector, APIC_SPURIOUS_VECTOR);
	}
	Ok(())
}

pub(super) fn inicializar_ap() -> Result<(), ErroApic> {
	let features = core::arch::x86_64::__cpuid(1);
	if features.edx & (1 << 9) == 0 {
		return Err(ErroApic::CpuSemApic);
	}

	let mut apic_base = unsafe { read_msr(APIC_BASE_MSR) };
	if X2APIC_MODE.load(Ordering::Acquire) {
		if features.ecx & (1 << 21) == 0 {
			return Err(ErroApic::CpuSemApic);
		}
		apic_base = (apic_base | APIC_GLOBAL_ENABLE | X2APIC_ENABLE) & !APIC_BASE_ADDRESS_MASK;
		unsafe {
			write_msr(APIC_BASE_MSR, apic_base);
			write_msr(X2APIC_SPURIOUS_VECTOR_MSR, u64::from(APIC_SPURIOUS_VECTOR));
		}
		return Ok(());
	}

	let physical_base = apic_base & APIC_BASE_ADDRESS_MASK;
	if physical_base == 0
		|| !virtual_memory::pagina_mapeada(physical_base)
			.map_err(ErroApic::MapeamentoVirtual)?
	{
		return Err(ErroApic::EnderecoInvalido);
	}
	apic_base = (apic_base | APIC_GLOBAL_ENABLE) & !X2APIC_ENABLE;
	unsafe {
		write_msr(APIC_BASE_MSR, apic_base);
		let spurious_vector = (physical_base as *mut u32).add(APIC_SPURIOUS_VECTOR_OFFSET);
		ptr::write_volatile(spurious_vector, APIC_SPURIOUS_VECTOR);
	}
	Ok(())
}

pub(super) fn id_local() -> Result<u32, ErroApic> {
	if X2APIC_MODE.load(Ordering::Acquire) {
		return Ok(unsafe { read_msr(X2APIC_ID_MSR) as u32 });
	}
	let base = APIC_PHYSICAL_BASE.load(Ordering::Acquire);
	if base == 0 {
		return Err(ErroApic::EnderecoInvalido);
	}
	Ok(unsafe {
		ptr::read_volatile((base as *const u32).add(XAPIC_ID_OFFSET)) >> 24
	})
}

pub(super) fn enviar_ipi(destino: u32, comando: u32) -> Result<(), ErroApic> {
	if X2APIC_MODE.load(Ordering::Acquire) {
		unsafe {
			write_msr(
				X2APIC_ICR_MSR,
				(u64::from(destino) << 32) | u64::from(comando),
			);
		}
	} else {
		if destino > u32::from(u8::MAX) {
			return Err(ErroApic::DestinoInvalido);
		}
		let base = APIC_PHYSICAL_BASE.load(Ordering::Acquire);
		if base == 0 {
			return Err(ErroApic::EnderecoInvalido);
		}
		unsafe {
			let registers = base as *mut u32;
			ptr::write_volatile(
				registers.add(XAPIC_ICR_HIGH_OFFSET),
				destino << 24,
			);
			ptr::write_volatile(registers.add(XAPIC_ICR_LOW_OFFSET), comando);
		}
	}
	esperar_icr()
}

pub(super) fn configurar_nmi_lint(
	lint_mask: u32,
	lint0_flags: u16,
	lint1_flags: u16,
) -> Result<usize, ErroApic> {
	if lint_mask & !0b11 != 0 {
		return Err(ErroApic::LintNmiInvalido);
	}
	let lint0 = if lint_mask & 1 != 0 {
		Some(configurar_flags_nmi(lint0_flags)?)
	} else {
		None
	};
	let lint1 = if lint_mask & 2 != 0 {
		Some(configurar_flags_nmi(lint1_flags)?)
	} else {
		None
	};

	if let Some(flags) = lint0 {
		configurar_lint(0, flags)?;
	}
	if let Some(flags) = lint1 {
		configurar_lint(1, flags)?;
	}
	Ok(lint_mask.count_ones() as usize)
}

fn configurar_flags_nmi(flags: u16) -> Result<u32, ErroApic> {
	if flags & !0x0f != 0 {
		return Err(ErroApic::FlagsNmiInvalidas);
	}
	let polarity = flags & 0b11;
	let trigger = (flags >> 2) & 0b11;
	if polarity == 0b10 || trigger == 0b10 {
		return Err(ErroApic::FlagsNmiInvalidas);
	}
	Ok((if polarity == 0b11 {
		LVT_POLARITY_ACTIVE_LOW
	} else {
		0
	}) | (if trigger == 0b11 {
		LVT_TRIGGER_LEVEL
	} else {
		0
	}))
}

fn configurar_lint(lint: u8, electrical_flags: u32) -> Result<(), ErroApic> {
	let (xapic_offset, x2apic_msr) = match lint {
		0 => (XAPIC_LVT_LINT0_OFFSET, X2APIC_LVT_LINT0_MSR),
		1 => (XAPIC_LVT_LINT1_OFFSET, X2APIC_LVT_LINT1_MSR),
		_ => return Err(ErroApic::LintNmiInvalido),
	};
	let mask = LVT_DELIVERY_MODE_MASK
		| LVT_POLARITY_ACTIVE_LOW
		| LVT_TRIGGER_LEVEL
		| LVT_MASKED;
	if X2APIC_MODE.load(Ordering::Acquire) {
		unsafe {
			let value = read_msr(x2apic_msr) as u32;
			write_msr(
				x2apic_msr,
				u64::from((value & !mask) | LVT_DELIVERY_MODE_NMI | electrical_flags),
			);
		}
		return Ok(());
	}

	let base = APIC_PHYSICAL_BASE.load(Ordering::Acquire);
	if base == 0 {
		return Err(ErroApic::EnderecoInvalido);
	}
	unsafe {
		let register = (base as *mut u32).add(xapic_offset);
		let value = ptr::read_volatile(register);
		ptr::write_volatile(
			register,
			(value & !mask) | LVT_DELIVERY_MODE_NMI | electrical_flags,
		);
	}
	Ok(())
}

fn esperar_icr() -> Result<(), ErroApic> {
	for _ in 0..MAX_IPI_POLLS {
		let pending = if X2APIC_MODE.load(Ordering::Acquire) {
			unsafe { read_msr(X2APIC_ICR_MSR) as u32 & ICR_DELIVERY_PENDING != 0 }
		} else {
			let base = APIC_PHYSICAL_BASE.load(Ordering::Acquire);
			if base == 0 {
				return Err(ErroApic::EnderecoInvalido);
			}
			unsafe {
				ptr::read_volatile((base as *const u32).add(XAPIC_ICR_LOW_OFFSET))
					& ICR_DELIVERY_PENDING
					!= 0
			}
		};
		if !pending {
			return Ok(());
		}
		core::hint::spin_loop();
	}
	Err(ErroApic::IpiNaoConcluido)
}

unsafe fn read_msr(msr: u32) -> u64 {
	let low: u32;
	let high: u32;
	core::arch::asm!(
		"rdmsr",
		in("ecx") msr,
		out("eax") low,
		out("edx") high,
		options(nomem, nostack, preserves_flags)
	);
	(u64::from(high) << 32) | u64::from(low)
}

unsafe fn write_msr(msr: u32, value: u64) {
	core::arch::asm!(
		"wrmsr",
		in("ecx") msr,
		in("eax") value as u32,
		in("edx") (value >> 32) as u32,
		options(nomem, nostack, preserves_flags)
	);
}

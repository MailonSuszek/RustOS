// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.8.2026
// Funcao: Inicializa e configura o controlador local APIC.
use core::ptr;

use super::virtual_memory;

const APIC_BASE_MSR: u32 = 0x1b;
const APIC_BASE_ADDRESS_MASK: u64 = 0x0000_000f_ffff_f000;
const APIC_GLOBAL_ENABLE: u64 = 1 << 11;
const X2APIC_ENABLE: u64 = 1 << 10;
const APIC_SPURIOUS_VECTOR: u32 = 0x1ff;
const APIC_SPURIOUS_VECTOR_OFFSET: usize = 0xf0 / core::mem::size_of::<u32>();
const X2APIC_SPURIOUS_VECTOR_MSR: u32 = 0x80f;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroApic {
	CpuSemApic,
	EnderecoInvalido,
	MapeamentoVirtual(virtual_memory::ErroVirtual),
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
	unsafe {
		write_msr(APIC_BASE_MSR, apic_base);
		let spurious_vector = (physical_base as *mut u32).add(APIC_SPURIOUS_VECTOR_OFFSET);
		ptr::write_volatile(spurious_vector, APIC_SPURIOUS_VECTOR);
	}
	Ok(())
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

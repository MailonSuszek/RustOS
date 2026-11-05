// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Configura e exibe os oito tipos da Page Attribute Table.
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, BootInfo};

const IA32_PAT: u32 = 0x277;
const PAT_ENTRIES: [u8; 8] = [6, 4, 7, 0, 1, 5, 7, 0];
const CR0_CACHE_DISABLE: u64 = 1 << 30;
const CR0_NOT_WRITE_THROUGH: u64 = 1 << 29;

static CONFIGURED: AtomicBool = AtomicBool::new(false);

pub(super) fn configurar() -> bool {
	if core::arch::x86_64::__cpuid(1).edx & ((1 << 12) | (1 << 16)) != (1 << 12) | (1 << 16) {
		return false;
	}
	let pat_value = PAT_ENTRIES
		.iter()
		.enumerate()
		.fold(0u64, |value, (index, memory_type)| {
			value | (u64::from(*memory_type) << (index * 8))
		});
	unsafe {
		let current_pat = read_msr(IA32_PAT);
		if current_pat != pat_value {
			core::arch::asm!("cli", options(nomem, nostack));
			core::arch::asm!("wbinvd", options(nostack, preserves_flags));
			let cr0: u64;
			core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
			core::arch::asm!(
				"mov cr0, {}",
				in(reg) (cr0 | CR0_CACHE_DISABLE) & !CR0_NOT_WRITE_THROUGH,
				options(nomem, nostack, preserves_flags)
			);
			core::arch::asm!("wbinvd", options(nostack, preserves_flags));
			write_msr(IA32_PAT, pat_value);
			core::arch::asm!("wbinvd", options(nostack, preserves_flags));
			core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack, preserves_flags));
		}
	}
	CONFIGURED.store(true, Ordering::Relaxed);
	true
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let message = if CONFIGURED.load(Ordering::Relaxed) {
		"PAT: WB, WT, UC-, UC, WC, WP, UC-, UC"
	} else {
		"PAT: firmware preservado"
	};
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} {}",
		line_number,
		message,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
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

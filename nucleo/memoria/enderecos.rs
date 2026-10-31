// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.6.2026
// Funcao: Interpreta e exibe os intervalos de memoria informados pelo mapa UEFI.
use core::fmt::{self, Write};
use core::ptr;

use super::{draw_scrolling_text, BootInfo};

const PAGE_SIZE: u64 = 4096;
const MEMORY_DESCRIPTOR_MIN_SIZE: usize = 40;
const FIRST_ADDRESS_LINE: usize = 5;
pub(super) const LINE_NUMBER_WIDTH: usize = 4;
const USAGE_LABEL_WIDTH: usize = 30;
const LINE_DELAY_MILLISECONDS: u64 = 25;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroMapa {
	MapaAusente,
	DescritorInvalido,
	EnderecoInvalido,
	LinhaMuitoLonga,
}

pub(super) fn exibir(boot_info: &BootInfo) -> Result<(usize, usize), ErroMapa> {
	if boot_info.memory_map.is_null() || boot_info.memory_map_size == 0 {
		return Err(ErroMapa::MapaAusente);
	}
	if boot_info.memory_descriptor_size < MEMORY_DESCRIPTOR_MIN_SIZE
		|| boot_info.memory_map_size % boot_info.memory_descriptor_size != 0
	{
		return Err(ErroMapa::DescritorInvalido);
	}

	let mut y = 38;
	draw_scrolling_text(
		boot_info,
		&mut y,
		"{   4} Lista de endereços da memoria RAM",
	);
	let tsc_frequency = tsc_frequency_hz();
	aguardar_linha(tsc_frequency);

	let descriptor_count = boot_info.memory_map_size / boot_info.memory_descriptor_size;
	let mut line_number = FIRST_ADDRESS_LINE;
	for index in 0..descriptor_count {
		let offset = index
			.checked_mul(boot_info.memory_descriptor_size)
			.ok_or(ErroMapa::DescritorInvalido)?;
		let descriptor = unsafe { boot_info.memory_map.add(offset) };
		let memory_type = unsafe { ptr::read_unaligned(descriptor.cast::<u32>()) };

		if matches!(memory_type, 11 | 12) {
			continue;
		}

		let physical_start =
			unsafe { ptr::read_unaligned(descriptor.add(8).cast::<u64>()) };
		let page_count = unsafe { ptr::read_unaligned(descriptor.add(24).cast::<u64>()) };
		if page_count == 0 {
			return Err(ErroMapa::EnderecoInvalido);
		}
		let byte_count = page_count
			.checked_mul(PAGE_SIZE)
			.ok_or(ErroMapa::EnderecoInvalido)?;
		let physical_end = physical_start
			.checked_add(byte_count)
			.and_then(|end| end.checked_sub(1))
			.ok_or(ErroMapa::EnderecoInvalido)?;

		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} {:016X}-{:016X} {:<usage_width$}",
			line_number,
			physical_start,
			physical_end,
			uso_memoria(memory_type),
			width = LINE_NUMBER_WIDTH,
			usage_width = USAGE_LABEL_WIDTH
		)
		.map_err(|_| ErroMapa::LinhaMuitoLonga)?;
		let text = line.como_str().map_err(|_| ErroMapa::LinhaMuitoLonga)?;
		draw_scrolling_text(boot_info, &mut y, text);
		aguardar_linha(tsc_frequency);
		line_number = line_number
			.checked_add(1)
			.ok_or(ErroMapa::EnderecoInvalido)?;
	}

	Ok((y, line_number))
}

fn tsc_frequency_hz() -> Option<u64> {
	let maximum_leaf = core::arch::x86_64::__cpuid(0).eax;
	if maximum_leaf >= 0x15 {
		let frequency = core::arch::x86_64::__cpuid(0x15);
		if frequency.eax != 0 && frequency.ebx != 0 && frequency.ecx != 0 {
			return u64::from(frequency.ecx)
				.checked_mul(u64::from(frequency.ebx))
				.map(|cycles| cycles / u64::from(frequency.eax))
				.filter(|cycles| *cycles != 0);
		}
	}

	if maximum_leaf >= 0x16 {
		let frequency = core::arch::x86_64::__cpuid(0x16).eax;
		if frequency != 0 {
			return u64::from(frequency).checked_mul(1_000_000);
		}
	}
	None
}

fn aguardar_linha(tsc_frequency: Option<u64>) {
	if let Some(cycles_per_second) = tsc_frequency {
		if let Some(cycles_to_wait) = cycles_per_second
			.checked_mul(LINE_DELAY_MILLISECONDS)
			.map(|cycles| cycles / 1_000)
		{
			let start = unsafe { core::arch::x86_64::_rdtsc() };
			while unsafe { core::arch::x86_64::_rdtsc() }.wrapping_sub(start) < cycles_to_wait {
				core::hint::spin_loop();
			}
			return;
		}
	}

	for _ in 0..4_000_000 {
		core::hint::spin_loop();
	}
}

fn uso_memoria(memory_type: u32) -> &'static str {
	match memory_type {
		0 => "usada reservada",
		1 => "usada codigo carregador",
		2 => "usada dados carregador",
		3 => "usada codigo inicio",
		4 => "usada dados inicio",
		5 => "usada codigo tempo de execução",
		6 => "usada dados tempo de execução",
		7 => "livre RAM",
		8 => "usada RAM inutilizavel",
		9 => "usada acpi recuperavel",
		10 => "usada acpi preservada",
		13 => "usada codigo pal",
		14 => "usada memoria persistente",
		15 => "usada memoria nao aceita",
		_ => "usada reservada",
	}
}

pub(super) struct LinhaBuffer {
	bytes: [u8; 96],
	len: usize,
}

impl LinhaBuffer {
	pub(super) const fn novo() -> Self {
		Self {
			bytes: [0; 96],
			len: 0,
		}
	}

	pub(super) fn como_str(&self) -> Result<&str, fmt::Error> {
		core::str::from_utf8(&self.bytes[..self.len]).map_err(|_| fmt::Error)
	}
}

impl Write for LinhaBuffer {
	fn write_str(&mut self, text: &str) -> fmt::Result {
		let end = self.len.checked_add(text.len()).ok_or(fmt::Error)?;
		let target = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
		target.copy_from_slice(text.as_bytes());
		self.len = end;
		Ok(())
	}
}
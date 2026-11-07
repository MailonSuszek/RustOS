// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.13.2026
// Funcao: Interpreta e exibe os intervalos de memoria informados pelo mapa UEFI.
use core::fmt::{self, Write};
use core::ptr;

use super::{draw_scrolling_text, BootInfo};

const PAGE_SIZE: u64 = 4096;
const MEMORY_DESCRIPTOR_MIN_SIZE: usize = 40;
pub(super) const LINE_NUMBER_WIDTH: usize = 4;
pub(super) const LINE_BUFFER_CAPACITY: usize = 96;
const USAGE_LABEL_WIDTH: usize = 30;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroMapa {
	MapaAusente,
	DescritorInvalido,
	EnderecoInvalido,
	LinhaMuitoLonga,
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	mut line_number: usize,
) -> Result<usize, ErroMapa> {
	if boot_info.memory_map.is_null() || boot_info.memory_map_size == 0 {
		return Err(ErroMapa::MapaAusente);
	}
	if boot_info.memory_descriptor_size < MEMORY_DESCRIPTOR_MIN_SIZE
		|| boot_info.memory_map_size % boot_info.memory_descriptor_size != 0
	{
		return Err(ErroMapa::DescritorInvalido);
	}

	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Lista de intervalos físicos de memoria",
		line_number,
		width = LINE_NUMBER_WIDTH
	)
	.map_err(|_| ErroMapa::LinhaMuitoLonga)?;
	draw_scrolling_text(
		boot_info,
		y,
		line.como_str().map_err(|_| ErroMapa::LinhaMuitoLonga)?,
	);
	line_number = line_number.checked_add(1).ok_or(ErroMapa::EnderecoInvalido)?;

	let descriptor_count = boot_info.memory_map_size / boot_info.memory_descriptor_size;
	let mut previous = None;
	for _ in 0..descriptor_count {
		let mut next = None;
		for index in 0..descriptor_count {
			let offset = index
				.checked_mul(boot_info.memory_descriptor_size)
				.ok_or(ErroMapa::DescritorInvalido)?;
			let descriptor = unsafe { boot_info.memory_map.add(offset) };
			let physical_start =
				unsafe { ptr::read_unaligned(descriptor.add(8).cast::<u64>()) };
			let candidate = (physical_start, index);
			if previous.is_some_and(|last| candidate <= last) {
				continue;
			}
			if next.is_none_or(|current| candidate < current) {
				next = Some(candidate);
			}
		}
		let (physical_start, index) = next.ok_or(ErroMapa::DescritorInvalido)?;
		previous = Some((physical_start, index));
		let offset = index
			.checked_mul(boot_info.memory_descriptor_size)
			.ok_or(ErroMapa::DescritorInvalido)?;
		let descriptor = unsafe { boot_info.memory_map.add(offset) };
		let memory_type = unsafe { ptr::read_unaligned(descriptor.cast::<u32>()) };

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
		draw_scrolling_text(boot_info, y, text);
		line_number = line_number
			.checked_add(1)
			.ok_or(ErroMapa::EnderecoInvalido)?;
	}

	Ok(line_number)
}

fn uso_memoria(memory_type: u32) -> &'static str {
	match memory_type {
		0 => "reservada firmware",
		1 => "reservada codigo carregador",
		2 => "reservada dados carregador",
		3 => "reservada codigo inicio",
		4 => "reservada dados inicio",
		5 => "reservada codigo runtime",
		6 => "reservada dados runtime",
		7 => "disponivel RAM",
		8 => "reservada RAM inutilizavel",
		9 => "reservada ACPI recuperavel",
		10 => "reservada ACPI preservada",
		11 => "reservada MMIO",
		12 => "reservada I/O",
		13 => "reservada codigo PAL",
		14 => "reservada memoria persistente",
		15 => "reservada memoria nao aceita",
		_ => "reservada firmware",
	}
}

pub(super) struct LinhaBuffer {
	bytes: [u8; LINE_BUFFER_CAPACITY],
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

	pub(super) fn tamanho(&self) -> usize {
		self.len
	}

	pub(super) fn limpar(&mut self) {
		self.len = 0;
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
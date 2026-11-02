// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Exibe fabricante, modelo e BIOS do computador capturados via DMI/SMBIOS.
use core::fmt::{self, Write};

use super::super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::super::{draw_scrolling_text, BootInfo};

pub(crate) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line_number = line_number;
	line_number = escrever(
		boot_info,
		y,
		line_number,
		"Fabricante",
		&boot_info.dmi_fabricante,
		boot_info.dmi_fabricante_len,
	)?;
	line_number = escrever(
		boot_info,
		y,
		line_number,
		"Modelo",
		&boot_info.dmi_modelo,
		boot_info.dmi_modelo_len,
	)?;
	escrever(
		boot_info,
		y,
		line_number,
		"BIOS",
		&boot_info.dmi_bios,
		boot_info.dmi_bios_len,
	)
}

fn escrever(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
	label: &str,
	text: &[u8],
	length: usize,
) -> Result<usize, fmt::Error> {
	let bytes = &text[..length.min(text.len())];
	let value = if bytes.is_empty() {
		"desconhecido"
	} else {
		core::str::from_utf8(bytes).unwrap_or("desconhecido")
	};
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} {}: {}",
		line_number,
		label,
		value,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

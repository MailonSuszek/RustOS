// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Exibe os enderecos das tabelas de configuracao UEFI recebidos no boot.
use core::fmt::{self, Write};

use super::enderecos::{LinhaBuffer, LINE_BUFFER_CAPACITY, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, BootInfo, MAX_FIRMWARE_TABLES};

const TABLE_KIND_ACPI_10: u32 = 1;
const TABLE_KIND_ACPI_20: u32 = 2;
const TABLE_KIND_SMBIOS: u32 = 3;
const TABLE_KIND_SMBIOS3: u32 = 4;

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line_number = line_number;
	let mut line = LinhaBuffer::novo();
	iniciar_linha(&mut line, line_number)?;

	let table_count = boot_info.firmware_table_count.min(MAX_FIRMWARE_TABLES);
	if table_count == 0 {
		write!(&mut line, " nenhuma")?;
	} else {
		for table in &boot_info.firmware_tables[..table_count] {
			let name = match table.kind {
				TABLE_KIND_ACPI_10 => "ACPI1",
				TABLE_KIND_ACPI_20 => "ACPI2",
				TABLE_KIND_SMBIOS => "SMBIOS",
				TABLE_KIND_SMBIOS3 => "SMBIOS3",
				_ => "UEFI",
			};
			let item_length = name.len() + 3 + 16;
			let separator_length = usize::from(line.tamanho() > 0);
			if line.tamanho() + separator_length + item_length > LINE_BUFFER_CAPACITY {
				draw_scrolling_text(boot_info, y, line.como_str()?);
				line_number = line_number.checked_add(1).ok_or(fmt::Error)?;
				iniciar_linha(&mut line, line_number)?;
			}
			if line.tamanho() > 0 {
				write!(&mut line, " ")?;
			}
			write!(&mut line, "{}=0x{:016X}", name, table.address)?;
		}
	}

	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

fn iniciar_linha(line: &mut LinhaBuffer, line_number: usize) -> fmt::Result {
	line.limpar();
	write!(
		line,
		"{{{:>width$}}} UEFI tabelas:",
		line_number,
		width = LINE_NUMBER_WIDTH
	)
}

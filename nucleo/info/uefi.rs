// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Exibe a versao do UEFI informada pela tabela de sistema do firmware.
use core::fmt::{self, Write};

use super::super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::super::{draw_scrolling_text, BootInfo};

pub(crate) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let revision = boot_info.efi_revision;
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} EFI: UEFI {}.{:02}",
		line_number,
		revision >> 16,
		revision & 0xffff,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

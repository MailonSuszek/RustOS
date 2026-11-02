// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Exibe a versao do SMBIOS capturada pelo carregador UEFI.
use core::fmt::{self, Write};

use super::super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::super::{draw_scrolling_text, BootInfo};

pub(crate) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let version = boot_info.smbios_version;
	let major = (version >> 8) as u8;
	let minor = (version & 0xff) as u8;
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} SMBIOS V{}.{}",
		line_number,
		major,
		minor,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

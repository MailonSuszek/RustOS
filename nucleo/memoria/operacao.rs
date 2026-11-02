// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Confirma o estado dos slots e canais de memoria informados pelo SMBIOS.
use core::fmt::{self, Write};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, BootInfo};

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let slots_total = boot_info.memoria_slots_total;
	let slots_ocupados = boot_info.memoria_slots_ocupados;
	let canais_total = boot_info.memoria_canais_total;
	let canais_ocupados = boot_info.memoria_canais_ocupados;
	let mut line = LinhaBuffer::novo();
	if slots_total == 0 {
		write!(
			&mut line,
			"{{{:>width$}}} Barramento de memoria: dados indisponiveis",
			line_number,
			width = LINE_NUMBER_WIDTH
		)?;
	} else if slots_ocupados == slots_total && canais_ocupados == canais_total {
		write!(
			&mut line,
			"{{{:>width$}}} Barramento de memoria OK: {}/{} slots e {}/{} canais ocupados",
			line_number,
			slots_ocupados,
			slots_total,
			canais_ocupados,
			canais_total,
			width = LINE_NUMBER_WIDTH
		)?;
	} else {
		write!(
			&mut line,
			"{{{:>width$}}} Barramento de memoria: {}/{} slots e {}/{} canais ocupados",
			line_number,
			slots_ocupados,
			slots_total,
			canais_ocupados,
			canais_total,
			width = LINE_NUMBER_WIDTH
		)?;
	}
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

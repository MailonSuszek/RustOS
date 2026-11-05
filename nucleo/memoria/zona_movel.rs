// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Reserva a metade superior da RAM como zona movel para dados realocaveis.
use core::fmt::{self, Write};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{bloco, draw_scrolling_text, paginas, BootInfo};

const TWO_MIB: u64 = 0x20_0000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroZonaMovel {
	Ram(bloco::ErroBloco),
	Reserva(paginas::ErroPaginas),
}

/// Guarda a metade superior da RAM para dados moveis, fora do alcance da RAM comum.
pub(super) fn inicializar(boot_info: &BootInfo) -> Result<(), ErroZonaMovel> {
	let topo = bloco::topo_da_ram(boot_info).map_err(ErroZonaMovel::Ram)?;
	let inicio = (topo / 2) & !(TWO_MIB - 1);
	if inicio == 0 {
		return Err(ErroZonaMovel::Ram(bloco::ErroBloco::RamAusente));
	}
	paginas::reservar(inicio, topo - inicio).map_err(ErroZonaMovel::Reserva)
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Início da zona móvel para a memoria RAM",
		line_number,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

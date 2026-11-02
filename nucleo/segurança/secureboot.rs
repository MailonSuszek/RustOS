// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Exibe se o Secure Boot estava ativo quando o carregador UEFI entregou o controle.
use core::fmt::Write;

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, BootInfo};

const SECURE_BOOT_ATIVO: u32 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroSecureBoot {
	Linha,
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, ErroSecureBoot> {
	let estado = if boot_info.secure_boot == SECURE_BOOT_ATIVO {
		"ativo"
	} else {
		"desativado"
	};
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Secureboot {}",
		line_number,
		estado,
		width = LINE_NUMBER_WIDTH
	)
	.map_err(|_| ErroSecureBoot::Linha)?;
	draw_scrolling_text(boot_info, y, line.como_str().map_err(|_| ErroSecureBoot::Linha)?);
	line_number.checked_add(1).ok_or(ErroSecureBoot::Linha)
}

// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.14.2026
// Funcao: Divide a RAM em zonas de hardware e lista cada zona em uma linha.
use core::fmt::{self, Write};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{bloco, draw_scrolling_text, BootInfo};

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;
const DMA_END: u64 = 16 * MIB;
const DMA32_END: u64 = 4 * GIB;

/// Faixa de RAM destinada a um tipo de trabalho do hardware.
struct Zona {
	nome: &'static str,
	inicio: u64,
	fim: u64,
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let topo = bloco::topo_da_ram(boot_info).map_err(|_| fmt::Error)?;
	let zonas = [
		Zona {
			nome: "DMA",
			inicio: 0,
			fim: DMA_END.min(topo),
		},
		Zona {
			nome: "DMA32",
			inicio: DMA_END,
			fim: DMA32_END.min(topo),
		},
		Zona {
			nome: "Normal",
			inicio: DMA32_END,
			fim: topo,
		},
	];

	let mut line_number = line_number;
	for zona in &zonas {
		let mut line = LinhaBuffer::novo();
		if zona.fim > zona.inicio {
			write!(
				&mut line,
				"{{{:>width$}}} Zona {}: 0x{:016X} - 0x{:016X} ({} MB)",
				line_number,
				zona.nome,
				zona.inicio,
				zona.fim - 1,
				(zona.fim - zona.inicio) / MIB,
				width = LINE_NUMBER_WIDTH
			)?;
		} else {
			write!(
				&mut line,
				"{{{:>width$}}} Zona {}: vazia",
				line_number,
				zona.nome,
				width = LINE_NUMBER_WIDTH
			)?;
		}
		draw_scrolling_text(boot_info, y, line.como_str()?);
		line_number = line_number.checked_add(1).ok_or(fmt::Error)?;
	}
	Ok(line_number)
}

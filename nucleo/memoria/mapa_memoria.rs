// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.13.2026
// Funcao: Reserva as areas de hardware que nao podem ser usadas como RAM comum.
use core::fmt::{self, Write};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, paginas, virtual_memory, BootInfo};

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const BIOS_ROM_END: u64 = 0x0010_0000;

/// Inicio da area de memoria de video VGA (abaixo de 1 MiB).
pub(super) const VIDEO_START: u64 = 0x000a_0000;
/// Fim da area de video; acima dela comeca a ROM da BIOS.
pub(super) const VIDEO_END: u64 = 0x000c_0000;
const VIDEO_SIZE: u64 = VIDEO_END - VIDEO_START;

struct FaixaInacessivel {
	inicio: u64,
	fim: u64,
	motivo: &'static str,
}

const FAIXAS_INACESSIVEIS: &[FaixaInacessivel] = &[
	FaixaInacessivel {
		inicio: 0,
		fim: paginas::PAGE_SIZE,
		motivo: "pagina zero",
	},
	FaixaInacessivel {
		inicio: VIDEO_START,
		fim: BIOS_ROM_END,
		motivo: "video e ROM da BIOS",
	},
	FaixaInacessivel {
		inicio: 0xfec0_0000,
		fim: 0xfec1_1000,
		motivo: "controle do chipset e IOAPIC",
	},
	FaixaInacessivel {
		inicio: 0xfed0_0000,
		fim: 0xfed0_1000,
		motivo: "temporizador HPET",
	},
	FaixaInacessivel {
		inicio: 0xfee0_0000,
		fim: 0xfee0_1000,
		motivo: "controlador Local APIC",
	},
	FaixaInacessivel {
		inicio: 0xff00_0000,
		fim: 0x1_0000_0000,
		motivo: "Flash da BIOS",
	},
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroExibicaoInacessiveis {
	Linha,
	Mapa(virtual_memory::ErroVirtual),
}

impl From<fmt::Error> for ErroExibicaoInacessiveis {
	fn from(_: fmt::Error) -> Self {
		Self::Linha
	}
}

/// Intervalo de memoria reservado para um dispositivo ou area de hardware.
struct Regiao {
	inicio: u64,
	fim: u64,
	rotulo: &'static str,
}

const REGIOES: &[Regiao] = &[
	Regiao {
		inicio: 0xfec0_0000,
		fim: 0xfec1_0fff,
		rotulo: "Reserva de controle do Chipset",
	},
	Regiao {
		inicio: 0xfec0_0000,
		fim: 0xfec0_0fff,
		rotulo: "Reserva de registradores do IOAPIC",
	},
	Regiao {
		inicio: 0xfed0_0000,
		fim: 0xfed0_0fff,
		rotulo: "Reserva de temporizador HPET",
	},
	Regiao {
		inicio: 0xfee0_0000,
		fim: 0xfee0_0fff,
		rotulo: "Reserva de registradores do Local APIC",
	},
	Regiao {
		inicio: 0xff00_0000,
		fim: 0xffff_ffff,
		rotulo: "Remover a memoria Flash da BIOS",
	},
	Regiao {
		inicio: 0x000c_0000,
		fim: BIOS_ROM_END - 1,
		rotulo: "Proteção da ROM da BIOS",
	},
];

fn tamanho(regiao: &Regiao) -> u64 {
	regiao.fim - regiao.inicio + 1
}

/// Impede que as areas de hardware sejam entregues pelo alocador como RAM comum.
pub(super) fn reservar() -> Result<(), paginas::ErroPaginas> {
	paginas::reservar(VIDEO_START, VIDEO_SIZE)?;
	for regiao in REGIOES {
		paginas::reservar(regiao.inicio, tamanho(regiao))?;
	}
	Ok(())
}

/// Remove o mapeamento das areas reservadas, tornando-as inacessiveis.
pub(super) fn desmapear() -> Result<(), virtual_memory::ErroVirtual> {
	for faixa in FAIXAS_INACESSIVEIS {
		virtual_memory::desmapear_intervalo(faixa.inicio, faixa.fim)?;
	}
	Ok(())
}

pub(super) fn exibir_paginas_inacessiveis(
	boot_info: &BootInfo,
	y: &mut usize,
	mut line_number: usize,
) -> Result<usize, ErroExibicaoInacessiveis> {
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Paginas sem acesso pelo mapa identidade",
		line_number,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number = line_number.checked_add(1).ok_or(ErroExibicaoInacessiveis::Linha)?;

	for faixa in FAIXAS_INACESSIVEIS {
		let mut page_count = 0u64;
		let mut address = faixa.inicio;
		while address < faixa.fim {
			if !virtual_memory::pagina_mapeada(address)
				.map_err(ErroExibicaoInacessiveis::Mapa)?
			{
				page_count = page_count
					.checked_add(1)
					.ok_or(ErroExibicaoInacessiveis::Linha)?;
			}
			address = address
				.checked_add(paginas::PAGE_SIZE)
				.ok_or(ErroExibicaoInacessiveis::Linha)?;
		}
		if page_count == 0 {
			continue;
		}
		let page_label = if page_count == 1 {
			"pagina"
		} else {
			"paginas"
		};
		line.limpar();
		write!(
			&mut line,
			"{{{:>width$}}} {} {} ({} KB) sem acesso: {}",
			line_number,
			page_count,
			page_label,
			page_count * (paginas::PAGE_SIZE / 1024),
			faixa.motivo,
			width = LINE_NUMBER_WIDTH
		)?;
		draw_scrolling_text(boot_info, y, line.como_str()?);
		line_number = line_number
			.checked_add(1)
			.ok_or(ErroExibicaoInacessiveis::Linha)?;
	}

	Ok(line_number)
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} {} KB reservado para memoria de video",
		line_number,
		VIDEO_SIZE / KIB,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	let mut line_number = line_number.checked_add(1).ok_or(fmt::Error)?;

	line.limpar();
	write!(
		&mut line,
		"{{{:>width$}}} {:016X} - {:016X} totalmente dedicado a memoria de video",
		line_number,
		VIDEO_START,
		VIDEO_END - 1,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number = line_number.checked_add(1).ok_or(fmt::Error)?;

	for regiao in REGIOES {
		line.limpar();
		escrever_regiao(&mut line, line_number, regiao)?;
		draw_scrolling_text(boot_info, y, line.como_str()?);
		line_number = line_number.checked_add(1).ok_or(fmt::Error)?;
	}
	Ok(line_number)
}

pub(super) fn exibir_reserva_video_bios(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line = LinhaBuffer::novo();
	line.limpar();
	write!(
		&mut line,
		"{{{:>width$}}} {:016X} - {:016X} ({} KB) Endereço reservado",
		line_number,
		VIDEO_START,
		BIOS_ROM_END - 1,
		(BIOS_ROM_END - VIDEO_START) / KIB,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

fn escrever_regiao(
	line: &mut LinhaBuffer,
	line_number: usize,
	regiao: &Regiao,
) -> fmt::Result {
	let bytes = tamanho(regiao);
	if bytes % MIB == 0 {
		write!(
			line,
			"{{{:>width$}}} {:016X} - {:016X} ({} MB) {}",
			line_number,
			regiao.inicio,
			regiao.fim,
			bytes / MIB,
			regiao.rotulo,
			width = LINE_NUMBER_WIDTH
		)
	} else {
		write!(
			line,
			"{{{:>width$}}} {:016X} - {:016X} ({} KB) {}",
			line_number,
			regiao.inicio,
			regiao.fim,
			bytes / KIB,
			regiao.rotulo,
			width = LINE_NUMBER_WIDTH
		)
	}
}

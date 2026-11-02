// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Inicializa e informa o estado de protecao da memoria do nucleo.
use core::fmt::Write;

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{
	aleatorio, draw_scrolling_text, interrupcoes, memoria_dinamica, paginas, regioes,
	virtual_memory, BootInfo,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroInicializacao {
	AlocadorPaginas(paginas::ErroPaginas),
	ReservaRegioes(paginas::ErroPaginas),
	ProtecaoVirtual(virtual_memory::ErroVirtual),
	Aleatoriedade(aleatorio::ErroAleatorio),
	HeapNucleo(memoria_dinamica::ErroHeap),
}

pub(super) fn inicializar_nucleo(
	boot_info: &BootInfo,
	rng_seed: [u8; 32],
	rng_seed_status: u32,
) -> Result<(), ErroInicializacao> {
	paginas::inicializar(
		boot_info.memory_map,
		boot_info.memory_map_size,
		boot_info.memory_descriptor_size,
	)
	.map_err(ErroInicializacao::AlocadorPaginas)?;
	regioes::reservar(boot_info).map_err(ErroInicializacao::ReservaRegioes)?;
	virtual_memory::inicializar(boot_info).map_err(ErroInicializacao::ProtecaoVirtual)?;
	aleatorio::inicializar(rng_seed, rng_seed_status)
		.map_err(ErroInicializacao::Aleatoriedade)?;
	interrupcoes::inicializar();
	memoria_dinamica::inicializar().map_err(ErroInicializacao::HeapNucleo)?;
	Ok(())
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, core::fmt::Error> {
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} RAM proteção ativo",
		line_number,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(core::fmt::Error)
}

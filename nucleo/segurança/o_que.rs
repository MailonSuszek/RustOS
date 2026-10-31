// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.5.2026
// Funcao: Inicializa e informa o estado de protecao da memoria do nucleo.
use core::fmt::Write;

use super::enderecos::LinhaBuffer;
use super::{draw_scrolling_text, heap, paginas, regioes, virtual_memory, BootInfo};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroInicializacao {
	AlocadorPaginas(paginas::ErroPaginas),
	ReservaRegioes(paginas::ErroPaginas),
	ProtecaoVirtual(virtual_memory::ErroVirtual),
	HeapNucleo(heap::ErroHeap),
}

pub(super) fn inicializar_nucleo(boot_info: &BootInfo) -> Result<(), ErroInicializacao> {
	paginas::inicializar(
		boot_info.memory_map,
		boot_info.memory_map_size,
		boot_info.memory_descriptor_size,
	)
	.map_err(ErroInicializacao::AlocadorPaginas)?;
	regioes::reservar(boot_info).map_err(ErroInicializacao::ReservaRegioes)?;
	virtual_memory::inicializar(boot_info).map_err(ErroInicializacao::ProtecaoVirtual)?;
	heap::inicializar().map_err(ErroInicializacao::HeapNucleo)?;
	Ok(())
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<(), core::fmt::Error> {
	let mut line = LinhaBuffer::novo();
	write!(&mut line, "{{{:>3}}} RAM proteção ativo", line_number)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	Ok(())
}

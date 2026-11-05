// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Cria um bloco virtual unico para toda a RAM e reserva a pagina do seu controle.
use core::fmt::{self, Write};
use core::ptr;

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, paginas, virtual_memory, BootInfo};

const BASE_VIRTUAL: u64 = 0x0000_0010_0000_0000;
const PAGE_SIZE: u64 = 4096;
const TWO_MIB: u64 = 0x20_0000;
const MAX_PHYSICAL_ADDRESS: u64 = 64 * 1024 * 1024 * 1024;
const MEMORY_DESCRIPTOR_MIN_SIZE: usize = 40;
const CONTROL_PAGES: usize = 1;
const RESERVED_MEMORY_TYPE: u32 = 0;
const MEMORY_MAPPED_IO: u32 = 11;
const MEMORY_MAPPED_IO_PORT: u32 = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroBloco {
	RamAusente,
	DescritorInvalido,
	Mapeamento(virtual_memory::ErroVirtual),
	ControleAusente,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ControleBloco {
	base_virtual: u64,
	base_fisica: u64,
	tamanho: u64,
	paginas: u64,
}

static mut ENDERECO_CONTROLE: u64 = 0;

pub(super) fn inicializar(boot_info: &BootInfo) -> Result<(), ErroBloco> {
	let tamanho = topo_da_ram(boot_info)?;
	virtual_memory::mapear_bloco(BASE_VIRTUAL, 0, tamanho)
		.map_err(ErroBloco::Mapeamento)?;
	let base_fisica =
		paginas::alocar(CONTROL_PAGES).map_err(|_| ErroBloco::ControleAusente)?;
	let controle = ControleBloco {
		base_virtual: BASE_VIRTUAL,
		base_fisica,
		tamanho,
		paginas: CONTROL_PAGES as u64,
	};
	unsafe {
		let destino = (BASE_VIRTUAL + base_fisica) as *mut ControleBloco;
		ptr::write_volatile(destino, controle);
		ENDERECO_CONTROLE = base_fisica;
	}
	Ok(())
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let endereco = unsafe { ENDERECO_CONTROLE };
	let controle = unsafe {
		ptr::read_volatile((BASE_VIRTUAL + endereco) as *const ControleBloco)
	};

	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Bloco virtual de RAM criado: 0x{:016X} - 0x{:016X}",
		line_number,
		controle.base_virtual,
		controle.base_virtual + controle.tamanho,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	let line_number = line_number.checked_add(1).ok_or(fmt::Error)?;

	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Controle do bloco reservado: 0x{:016X}",
		line_number,
		endereco,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

// Calcula o fim da RAM ignorando regioes reservadas e de dispositivo mapeado.
pub(super) fn topo_da_ram(boot_info: &BootInfo) -> Result<u64, ErroBloco> {
	if boot_info.memory_map.is_null()
		|| boot_info.memory_map_size == 0
		|| boot_info.memory_descriptor_size < MEMORY_DESCRIPTOR_MIN_SIZE
		|| boot_info.memory_map_size % boot_info.memory_descriptor_size != 0
	{
		return Err(ErroBloco::RamAusente);
	}

	let descriptor_count = boot_info.memory_map_size / boot_info.memory_descriptor_size;
	let mut topo = 0u64;
	for index in 0..descriptor_count {
		let offset = index
			.checked_mul(boot_info.memory_descriptor_size)
			.ok_or(ErroBloco::DescritorInvalido)?;
		let descriptor = unsafe { boot_info.memory_map.add(offset) };
		let memory_type = unsafe { ptr::read_unaligned(descriptor.cast::<u32>()) };
		if matches!(
			memory_type,
			RESERVED_MEMORY_TYPE | MEMORY_MAPPED_IO | MEMORY_MAPPED_IO_PORT
		) {
			continue;
		}

		let start = unsafe { ptr::read_unaligned(descriptor.add(8).cast::<u64>()) };
		let page_count = unsafe { ptr::read_unaligned(descriptor.add(24).cast::<u64>()) };
		let end = start
			.checked_add(
				page_count
					.checked_mul(PAGE_SIZE)
					.ok_or(ErroBloco::DescritorInvalido)?,
			)
			.ok_or(ErroBloco::DescritorInvalido)?;
		topo = topo.max(end.min(MAX_PHYSICAL_ADDRESS));
	}

	if topo == 0 {
		return Err(ErroBloco::RamAusente);
	}
	topo.checked_add(TWO_MIB - 1)
		.ok_or(ErroBloco::DescritorInvalido)
		.map(|arredondado| arredondado & !(TWO_MIB - 1))
}

// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.7.2026
// Funcao: Aloca, reserva e libera paginas fisicas usando o mapa UEFI.
use core::ptr;

const PAGE_SIZE: u64 = 4096;
const MAX_PHYSICAL_ADDRESS: u64 = 64 * 1024 * 1024 * 1024;
const MAX_PAGES: usize = (MAX_PHYSICAL_ADDRESS / PAGE_SIZE) as usize;
const BITMAP_WORDS: usize = MAX_PAGES / u64::BITS as usize;
const MEMORY_DESCRIPTOR_MIN_SIZE: usize = 40;
const EFI_CONVENTIONAL_MEMORY: u32 = 7;

static mut BITMAP: [u64; BITMAP_WORDS] = [0; BITMAP_WORDS];
static mut ALLOCATIONS: [u64; BITMAP_WORDS] = [0; BITMAP_WORDS];
static mut INITIALIZED: bool = false;
static mut SEARCH_CURSOR: usize = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroPaginas {
	MapaAusente,
	DescritorInvalido,
	EnderecoInvalido,
	MemoriaInsuficiente,
	PaginaInvalida,
	JaInicializado,
}

pub(super) fn inicializar(
	memory_map: *const u8,
	memory_map_size: usize,
	descriptor_size: usize,
) -> Result<(), ErroPaginas> {
	if unsafe { INITIALIZED } {
		return Err(ErroPaginas::JaInicializado);
	}
	if memory_map.is_null() || memory_map_size == 0 {
		return Err(ErroPaginas::MapaAusente);
	}
	if descriptor_size < MEMORY_DESCRIPTOR_MIN_SIZE || memory_map_size % descriptor_size != 0 {
		return Err(ErroPaginas::DescritorInvalido);
	}

	let descriptor_count = memory_map_size / descriptor_size;
	for index in 0..descriptor_count {
		let offset = index
			.checked_mul(descriptor_size)
			.ok_or(ErroPaginas::DescritorInvalido)?;
		let descriptor = unsafe { memory_map.add(offset) };
		let memory_type = unsafe { ptr::read_unaligned(descriptor.cast::<u32>()) };
		if memory_type != EFI_CONVENTIONAL_MEMORY {
			continue;
		}

		let start = unsafe { ptr::read_unaligned(descriptor.add(8).cast::<u64>()) };
		let page_count = unsafe { ptr::read_unaligned(descriptor.add(24).cast::<u64>()) };
		let end = start
			.checked_add(
				page_count
					.checked_mul(PAGE_SIZE)
					.ok_or(ErroPaginas::EnderecoInvalido)?,
			)
			.ok_or(ErroPaginas::EnderecoInvalido)?;
		marcar_intervalo(start, end, true)?;
	}

	reservar_intervalo(0, 0x10_0000)?;
	unsafe {
		INITIALIZED = true;
	}
	Ok(())
}

pub(super) fn alocar(page_count: usize) -> Result<u64, ErroPaginas> {
	if !inicializado() || page_count == 0 || page_count > MAX_PAGES {
		return Err(ErroPaginas::MemoriaInsuficiente);
	}

	let start = unsafe { SEARCH_CURSOR };
	let mut checked = 0;
	let mut candidate = start;
	while checked < MAX_PAGES {
		if candidate >= MAX_PAGES {
			candidate = 0;
		}
		if candidato_disponivel(candidate, page_count) {
			marcar_paginas(candidate, page_count, false);
			marcar_alocacoes(candidate, page_count, true);
			unsafe {
				SEARCH_CURSOR = (candidate + page_count) % MAX_PAGES;
			}
			return Ok((candidate as u64) * PAGE_SIZE);
		}
		candidate += 1;
		checked += 1;
	}

	Err(ErroPaginas::MemoriaInsuficiente)
}

pub(super) fn liberar(address: u64, page_count: usize) -> Result<(), ErroPaginas> {
	if !inicializado()
		|| address % PAGE_SIZE != 0
		|| page_count == 0
		|| address >= MAX_PHYSICAL_ADDRESS
	{
		return Err(ErroPaginas::PaginaInvalida);
	}
	let first = (address / PAGE_SIZE) as usize;
	let end = first
		.checked_add(page_count)
		.filter(|end| *end <= MAX_PAGES)
		.ok_or(ErroPaginas::PaginaInvalida)?;
	for page in first..end {
		if !pagina_alocada(page) {
			return Err(ErroPaginas::PaginaInvalida);
		}
	}
	marcar_paginas(first, page_count, true);
	marcar_alocacoes(first, page_count, false);
	unsafe {
		SEARCH_CURSOR = first;
	}
	Ok(())
}

pub(super) fn reservar(address: u64, size: u64) -> Result<(), ErroPaginas> {
	if !inicializado() {
		return Err(ErroPaginas::PaginaInvalida);
	}
	reservar_intervalo(address, size)
}

fn reservar_intervalo(start: u64, size: u64) -> Result<(), ErroPaginas> {
	if size == 0 {
		return Ok(());
	}
	let end = start
		.checked_add(size)
		.ok_or(ErroPaginas::EnderecoInvalido)?;
	marcar_intervalo(start, end, false)
}

fn marcar_intervalo(start: u64, end: u64, available: bool) -> Result<(), ErroPaginas> {
	if start >= end {
		return Ok(());
	}
	let (first, last) = if available {
		(start
			.checked_add(PAGE_SIZE - 1)
			.ok_or(ErroPaginas::EnderecoInvalido)?
			/ PAGE_SIZE, end / PAGE_SIZE)
	} else {
		(start / PAGE_SIZE, end
			.checked_add(PAGE_SIZE - 1)
			.ok_or(ErroPaginas::EnderecoInvalido)?
			/ PAGE_SIZE)
	};
	if first >= MAX_PAGES as u64 {
		return Ok(());
	}
	let first = first as usize;
	let last = (last.min(MAX_PAGES as u64)) as usize;
	if first < last {
		marcar_paginas(first, last - first, available);
	}
	Ok(())
}

fn candidato_disponivel(first: usize, page_count: usize) -> bool {
	let Some(end) = first.checked_add(page_count) else {
		return false;
	};
	if end > MAX_PAGES {
		return false;
	}
	(first..end).all(pagina_disponivel)
}

fn pagina_disponivel(page: usize) -> bool {
	let word = page / u64::BITS as usize;
	let bit = page % u64::BITS as usize;
	unsafe { ptr::addr_of!(BITMAP).cast::<u64>().add(word).read() & (1u64 << bit) != 0 }
}

fn pagina_alocada(page: usize) -> bool {
	let word = page / u64::BITS as usize;
	let bit = page % u64::BITS as usize;
	unsafe {
		ptr::addr_of!(ALLOCATIONS).cast::<u64>().add(word).read() & (1u64 << bit) != 0
	}
}

fn marcar_paginas(first: usize, page_count: usize, available: bool) {
	for page in first..first + page_count {
		let word = page / u64::BITS as usize;
		let bit = page % u64::BITS as usize;
		let bitmap = unsafe { ptr::addr_of_mut!(BITMAP).cast::<u64>().add(word) };
		unsafe {
			let value = bitmap.read();
			bitmap.write(if available {
				value | (1u64 << bit)
			} else {
				value & !(1u64 << bit)
			});
		}
	}
}

fn marcar_alocacoes(first: usize, page_count: usize, allocated: bool) {
	for page in first..first + page_count {
		let word = page / u64::BITS as usize;
		let bit = page % u64::BITS as usize;
		let bitmap = unsafe { ptr::addr_of_mut!(ALLOCATIONS).cast::<u64>().add(word) };
		unsafe {
			let value = bitmap.read();
			bitmap.write(if allocated {
				value | (1u64 << bit)
			} else {
				value & !(1u64 << bit)
			});
		}
	}
}

fn inicializado() -> bool {
	unsafe { INITIALIZED }
}

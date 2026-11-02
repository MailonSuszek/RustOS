// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Reserva as regioes ocupadas pelo nucleo, framebuffer e dados de boot.
use core::ptr;

use super::{mapa_memoria, paginas, BootInfo};

unsafe extern "C" {
	static __kernel_start: u8;
	static __kernel_end: u8;
}

pub(super) fn reservar(boot_info: &BootInfo) -> Result<(), paginas::ErroPaginas> {
	mapa_memoria::reservar()?;

	let kernel_start = ptr::addr_of!(__kernel_start) as u64;
	let kernel_end = ptr::addr_of!(__kernel_end) as u64;
	paginas::reservar(kernel_start, kernel_end - kernel_start)?;

	let frame_buffer_size = boot_info
		.pixels_per_scan_line
		.checked_mul(boot_info.height)
		.and_then(|pixels| pixels.checked_mul(core::mem::size_of::<u32>()))
		.ok_or(paginas::ErroPaginas::EnderecoInvalido)?;
	paginas::reservar(boot_info.frame_buffer as u64, frame_buffer_size as u64)?;
	paginas::reservar(
		boot_info as *const BootInfo as u64,
		core::mem::size_of::<BootInfo>() as u64,
	)?;
	paginas::reservar(
		boot_info.memory_map as u64,
		boot_info.memory_map_size as u64,
	)
}

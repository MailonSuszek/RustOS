// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.6.2026
// Funcao: Ponto de entrada e nucleo freestanding do sistema operacional.
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::ptr;

#[path = "segurança/quando.rs"]
mod quando;
#[path = "segurança/o_que.rs"]
mod o_que;
#[path = "cpu/apic.rs"]
mod apic;
#[path = "cpu/idt.rs"]
mod idt;
#[path = "cpu/ioapic.rs"]
mod ioapic;
#[path = "cpu/interrupcoes.rs"]
mod interrupcoes;
#[path = "memoria/enderecos.rs"]
mod enderecos;
#[path = "memoria/memoria_dinamica.rs"]
mod memoria_dinamica;
#[path = "memoria/paginas.rs"]
mod paginas;
#[path = "memoria/regioes.rs"]
mod regioes;
#[path = "memoria/virtual.rs"]
mod virtual_memory;

const PIXEL_RGB_RESERVED_8: u32 = 0;
const PIXEL_BGR_RESERVED_8: u32 = 1;

#[repr(C)]
pub struct BootInfo {
	frame_buffer: *mut u32,
	width: usize,
	height: usize,
	pixels_per_scan_line: usize,
	pixel_format: u32,
	memory_map: *const u8,
	memory_map_size: usize,
	memory_descriptor_size: usize,
	acpi_rsdp: *const u8,
	efi_revision: u32,
}

#[no_mangle]
pub extern "C" fn kernel_main(boot_info: *const BootInfo) -> ! {
	if boot_info.is_null() {
		halt();
	}
	let boot_info = unsafe { &*boot_info };
	if boot_info.frame_buffer.is_null()
		|| boot_info.width == 0
		|| boot_info.height == 0
		|| boot_info.pixels_per_scan_line < boot_info.width
		|| !matches!(
			boot_info.pixel_format,
			PIXEL_RGB_RESERVED_8 | PIXEL_BGR_RESERVED_8
		)
	{
		halt();
	}

	if let Err(error) = o_que::inicializar_nucleo(boot_info) {
		let message = match error {
			o_que::ErroInicializacao::AlocadorPaginas(_) => "Erro alocador RAM",
			o_que::ErroInicializacao::ReservaRegioes(_) => "Erro reserva nucleo",
			o_que::ErroInicializacao::ProtecaoVirtual(_) => "Erro protecao virtual",
			o_que::ErroInicializacao::HeapNucleo(_) => "Erro heap nucleo",
		};
		draw_text(boot_info, 8, 48, message);
		halt();
	}

	if let Err(error) = quando::executar(boot_info) {
		let message = match error {
			quando::ErroOrdem::Apic(_) => "Erro APCI",
			quando::ErroOrdem::IoApic(_) => "Erro IOAPIC",
			_ => "Erro inicializacao",
		};
		draw_text(boot_info, 8, 48, message);
		halt();
	}
	halt()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	halt()
}

fn halt() -> ! {
	loop {
		core::hint::spin_loop();
	}
}

fn draw_text(boot_info: &BootInfo, mut x: usize, y: usize, text: &str) {
	for character in text.chars() {
		let glyph = glyph(character);
		for (row, bits) in glyph.iter().enumerate() {
			for column in 0..5 {
				if bits & (1 << (4 - column)) != 0 {
					draw_pixel(boot_info, x + column, y + row);
				}
			}
		}
		x += 6;
	}
}

fn draw_scrolling_text(boot_info: &BootInfo, y: &mut usize, text: &str) {
	const LINE_HEIGHT: usize = 10;
	const TEXT_HEIGHT: usize = 8;

	while y.saturating_add(TEXT_HEIGHT) > boot_info.height {
		if boot_info.height <= LINE_HEIGHT {
			*y = 0;
			break;
		}
		scroll_up(boot_info, LINE_HEIGHT);
		*y = y.saturating_sub(LINE_HEIGHT);
	}

	for (column, character) in text.chars().enumerate() {
		let glyph = glyph(character);
		for (row, bits) in glyph.iter().enumerate() {
			for pixel_column in 0..5 {
				if bits & (1 << (4 - pixel_column)) != 0 {
					draw_pixel(boot_info, 8 + column * 6 + pixel_column, *y + row);
				}
			}
		}
	}
	*y = y.saturating_add(LINE_HEIGHT);
}

fn scroll_up(boot_info: &BootInfo, amount: usize) {
	let shift = amount.min(boot_info.height);
	let remaining_height = boot_info.height - shift;
	for y in 0..remaining_height {
		for x in 0..boot_info.width {
			unsafe {
				let source = boot_info
					.frame_buffer
					.add((y + shift) * boot_info.pixels_per_scan_line + x);
				let destination = boot_info
					.frame_buffer
					.add(y * boot_info.pixels_per_scan_line + x);
				ptr::write_volatile(destination, ptr::read_volatile(source));
			}
		}
	}
	for y in remaining_height..boot_info.height {
		for x in 0..boot_info.width {
			unsafe {
				ptr::write_volatile(
					boot_info
						.frame_buffer
						.add(y * boot_info.pixels_per_scan_line + x),
					0,
				);
			}
		}
	}
}

fn draw_pixel(boot_info: &BootInfo, x: usize, y: usize) {
	if x >= boot_info.width || y >= boot_info.height {
		return;
	}
	let pixel = match boot_info.pixel_format {
		PIXEL_RGB_RESERVED_8 => 0x00ff_ffff,
		PIXEL_BGR_RESERVED_8 => 0x00ff_ffff,
		_ => return,
	};
	unsafe {
		ptr::write_volatile(
			boot_info
				.frame_buffer
				.add(y * boot_info.pixels_per_scan_line + x),
			pixel,
		);
	}
}

fn glyph(character: char) -> [u8; 8] {
	match character {
		'{' => [0b00110, 0b01000, 0b01000, 0b10000, 0b01000, 0b01000, 0b00110, 0],
		'}' => [0b01100, 0b00010, 0b00010, 0b00001, 0b00010, 0b00010, 0b01100, 0],
		'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0],
		'4' => [0b10010, 0b10010, 0b10010, 0b11111, 0b00010, 0b00010, 0b00010, 0],
		'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110, 0],
		'2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111, 0],
		'3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110, 0],
		'6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110, 0],
		'5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110, 0],
		'7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0],
		'8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110, 0],
		'9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100, 0],
		' ' => [0; 8],
		'.' => [0, 0, 0, 0, 0, 0b01100, 0b01100, 0],
		'-' => [0, 0, 0, 0b11111, 0, 0, 0, 0],
		':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0, 0],
		'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001, 0],
		'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110, 0],
		'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110, 0],
		'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111, 0],
		'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000, 0],
		'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111, 0],
		'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001, 0],
		'a' => [0, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111, 0],
		'b' => [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110, 0],
		'd' => [0, 0, 0b00001, 0b01101, 0b10011, 0b10001, 0b01111, 0],
		'g' => [0, 0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
		'm' => [0, 0, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101, 0],
		'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110, 0],
		'h' => [0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0],
		'x' => [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0],
		'/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000, 0],
		'n' => [0, 0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001, 0],
		'l' => [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0],
		'p' => [0, 0, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000],
		'c' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10000, 0b01110, 0],
		'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001, 0],
		'u' => [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101, 0],
		'r' => [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000, 0],
		's' => [0, 0, 0b01110, 0b10000, 0b01110, 0b00001, 0b11110, 0],
		't' => [0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00100, 0b00011, 0],
		'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110, 0],
		'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000, 0],
		'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111, 0],
		'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0],
		'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110, 0],
		'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110, 0],
		'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110, 0],
		'e' => [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110, 0],
		'v' => [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0],
		'o' => [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110, 0],
		'z' => [0, 0, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111, 0],
		'ç' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110, 0b00100],
		'ã' => [0b01010, 0b00101, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111],
		_ => [0b11111, 0b10001, 0b00010, 0b00100, 0b00100, 0, 0b00100, 0],
	}
}
// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.3.2026
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::ptr;

const PIXEL_RGB_RESERVED_8: u32 = 0;
const PIXEL_BGR_RESERVED_8: u32 = 1;

#[repr(C)]
pub struct BootInfo {
	frame_buffer: *mut u32,
	width: usize,
	height: usize,
	pixels_per_scan_line: usize,
	pixel_format: u32,
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

	draw_text(boot_info, 8, 8, "{1} RustOS Beta v0.1 Construção 0.3.2026");
	draw_text(boot_info, 8, 28, "{2} Caminho: nucleo/Rust");
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
					for dy in 0..2 {
						for dx in 0..2 {
							draw_pixel(boot_info, x + column * 2 + dx, y + row * 2 + dy);
						}
					}
				}
			}
		}
		x += 12;
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
		'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110, 0],
		'2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111, 0],
		'3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110, 0],
		'6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110, 0],
		'.' => [0, 0, 0, 0, 0, 0b01100, 0b01100, 0],
		' ' => [0; 8],
		':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0, 0],
		'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110, 0],
		'a' => [0, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111, 0],
		'm' => [0, 0, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101, 0],
		'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110, 0],
		'h' => [0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0],
		'/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000, 0],
		'n' => [0, 0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001, 0],
		'l' => [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0],
		'c' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10000, 0b01110, 0],
		'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001, 0],
		'u' => [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101, 0],
		'r' => [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000, 0],
		's' => [0, 0, 0b01110, 0b10000, 0b01110, 0b00001, 0b11110, 0],
		't' => [0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00100, 0b00011, 0],
		'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110, 0],
		'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110, 0],
		'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110, 0],
		'e' => [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110, 0],
		'v' => [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0],
		'o' => [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110, 0],
		'ç' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110, 0b00100],
		'ã' => [0b01010, 0b00101, 0, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111],
		_ => [0b11111, 0b10001, 0b00010, 0b00100, 0b00100, 0, 0b00100, 0],
	}
}
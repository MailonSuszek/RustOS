#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

type Handle = *mut c_void;
type Status = usize;

#[repr(C)]
struct SimpleTextOutputProtocol {
	reset: usize,
	output_string: extern "efiapi" fn(*mut SimpleTextOutputProtocol, *const u16) -> Status,
}

#[repr(C)]
struct SystemTable {
	header: [u8; 24],
	firmware_vendor: *mut u16,
	firmware_revision: u32,
	console_in_handle: Handle,
	console_in: *mut c_void,
	console_out_handle: Handle,
	console_out: *mut SimpleTextOutputProtocol,
}

const fn ascii_to_utf16<const N: usize>(bytes: &[u8; N]) -> [u16; N] {
	let mut result = [0; N];
	let mut index = 0;
	while index < N {
		result[index] = bytes[index] as u16;
		index += 1;
	}
	result
}

static MESSAGE: [u16; 27] = ascii_to_utf16(b"Bem-vindo ao RustOS Beta\r\n\0");

#[no_mangle]
extern "efiapi" fn efi_main(_image_handle: Handle, system_table: *mut SystemTable) -> Status {
	unsafe {
		let console = (*system_table).console_out;
		((*console).output_string)(console, MESSAGE.as_ptr());
	}
	loop {
		core::hint::spin_loop();
	}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {
		core::hint::spin_loop();
	}
}

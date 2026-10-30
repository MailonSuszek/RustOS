// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.2.2026
#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

type Handle = *mut c_void;
type Status = usize;

const SUCCESS: Status = 0;
const PIXEL_BLT_ONLY: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
	data1: u32,
	data2: u16,
	data3: u16,
	data4: [u8; 8],
}

#[repr(C)]
struct SystemTable {
	header: [u8; 24],
	firmware_vendor: *mut u16,
	firmware_revision: u32,
	console_in_handle: Handle,
	console_in: *mut c_void,
	console_out_handle: Handle,
	console_out: *mut c_void,
	standard_error_handle: Handle,
	standard_error: *mut c_void,
	runtime_services: *mut c_void,
	boot_services: *mut BootServices,
	number_of_table_entries: usize,
	configuration_table: *mut c_void,
}

type FreePool = extern "efiapi" fn(*mut c_void) -> Status;
type LocateProtocol = extern "efiapi" fn(*const Guid, *mut c_void, *mut *mut c_void) -> Status;

#[repr(C)]
struct BootServices {
	header: [u8; 24],
	_raise_tpl: usize,
	_restore_tpl: usize,
	_allocate_pages: usize,
	_free_pages: usize,
	_get_memory_map: usize,
	_allocate_pool: usize,
	free_pool: FreePool,
	_create_event: usize,
	_set_timer: usize,
	_wait_for_event: usize,
	_signal_event: usize,
	_close_event: usize,
	_check_event: usize,
	_install_protocol_interface: usize,
	_reinstall_protocol_interface: usize,
	_uninstall_protocol_interface: usize,
	_handle_protocol: usize,
	_reserved: usize,
	_register_protocol_notify: usize,
	_locate_handle: usize,
	_locate_device_path: usize,
	_install_configuration_table: usize,
	_load_image: usize,
	_start_image: usize,
	_exit: usize,
	_unload_image: usize,
	_exit_boot_services: usize,
	_get_next_monotonic_count: usize,
	_stall: usize,
	_set_watchdog_timer: usize,
	_connect_controller: usize,
	_disconnect_controller: usize,
	_open_protocol: usize,
	_close_protocol: usize,
	_open_protocol_information: usize,
	_protocols_per_handle: usize,
	_locate_handle_buffer: usize,
	locate_protocol: LocateProtocol,
}

#[repr(C)]
struct GraphicsOutputProtocol {
	query_mode: extern "efiapi" fn(
		*mut GraphicsOutputProtocol,
		u32,
		*mut usize,
		*mut *mut GraphicsOutputModeInformation,
	) -> Status,
	set_mode: extern "efiapi" fn(*mut GraphicsOutputProtocol, u32) -> Status,
	blt: extern "efiapi" fn(
		*mut GraphicsOutputProtocol,
		*mut BltPixel,
		u32,
		usize,
		usize,
		usize,
		usize,
		usize,
		usize,
	) -> Status,
	mode: *mut GraphicsOutputProtocolMode,
}

#[repr(C)]
struct GraphicsOutputProtocolMode {
	max_mode: u32,
	mode: u32,
	info: *mut GraphicsOutputModeInformation,
	size_of_info: usize,
	frame_buffer_base: u64,
	frame_buffer_size: usize,
}

#[repr(C)]
struct GraphicsOutputModeInformation {
	version: u32,
	horizontal_resolution: u32,
	vertical_resolution: u32,
	pixel_format: u32,
	pixel_information: [u32; 4],
	pixels_per_scan_line: u32,
}

#[repr(C)]
struct BltPixel {
	blue: u8,
	green: u8,
	red: u8,
	reserved: u8,
}

const GRAPHICS_OUTPUT_GUID: Guid = Guid {
	data1: 0x9042a9de,
	data2: 0x23dc,
	data3: 0x4a38,
	data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[no_mangle]
extern "efiapi" fn efi_main(_image_handle: Handle, system_table: *mut SystemTable) -> Status {
	if system_table.is_null() {
		halt();
	}

	unsafe {
		set_black_video_mode(system_table);
	}

	loop {
		core::hint::spin_loop();
	}
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

unsafe fn set_black_video_mode(system_table: *mut SystemTable) {
	let boot_services = (*system_table).boot_services;
	if boot_services.is_null() {
		halt();
	}

	let mut protocol: *mut c_void = core::ptr::null_mut();
	let status = ((*boot_services).locate_protocol)(
		&GRAPHICS_OUTPUT_GUID,
		core::ptr::null_mut(),
		&mut protocol,
	);
	if status != SUCCESS || protocol.is_null() {
		halt();
	}
	let graphics = protocol as *mut GraphicsOutputProtocol;
	if (*graphics).mode.is_null() {
		halt();
	}

	let max_mode = (*(*graphics).mode).max_mode;
	let mut selected_mode = None;
	let mut mode_number = 0;
	while mode_number < max_mode {
		let mut info_size = 0;
		let mut info = core::ptr::null_mut();
		let status = ((*graphics).query_mode)(graphics, mode_number, &mut info_size, &mut info);
		if status != SUCCESS || info.is_null() {
			halt();
		}
		let matches =
			(*info).horizontal_resolution == 640 && (*info).vertical_resolution == 480;
		let free_status = ((*boot_services).free_pool)(info as *mut c_void);
		if free_status != SUCCESS {
			halt();
		}
		if matches {
			selected_mode = Some(mode_number);
			break;
		}
		mode_number += 1;
	}

	let Some(selected_mode) = selected_mode else {
		halt();
	};
	let status = ((*graphics).set_mode)(graphics, selected_mode);
	if status != SUCCESS {
		halt();
	}

	let mode = &mut *(*graphics).mode;
	if mode.info.is_null() {
		halt();
	}
	let info = &*mode.info;
	if info.pixel_format != PIXEL_BLT_ONLY {
		let pixels_per_scan_line = info.pixels_per_scan_line as usize;
		let Some(frame_buffer_pixels) = pixels_per_scan_line.checked_mul(480) else {
			halt();
		};
		let Some(frame_buffer_bytes) = frame_buffer_pixels.checked_mul(4) else {
			halt();
		};
		if pixels_per_scan_line < 640
			|| mode.frame_buffer_base == 0
			|| mode.frame_buffer_size < frame_buffer_bytes
		{
			halt();
		}

		let frame_buffer = mode.frame_buffer_base as *mut u32;
		for y in 0..480 {
			for x in 0..640 {
				core::ptr::write_volatile(frame_buffer.add(y * pixels_per_scan_line + x), 0);
			}
		}
		return;
	}

	let mut black = BltPixel {
		blue: 0,
		green: 0,
		red: 0,
		reserved: 0,
	};
	let status = ((*graphics).blt)(
		graphics,
		&mut black,
		0,
		0,
		0,
		0,
		640,
		480,
		0,
	);
	if status != SUCCESS {
		halt();
	}
}
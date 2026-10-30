// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.3.2026
#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

type Handle = *mut c_void;
type Status = usize;

const SUCCESS: Status = 0;
const BUFFER_TOO_SMALL: Status = (1usize << (usize::BITS - 1)) | 5;
const LOADER_DATA: u32 = 2;
const FILE_MODE_READ: u64 = 1;

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
	console_out: *mut SimpleTextOutputProtocol,
	standard_error_handle: Handle,
	standard_error: *mut SimpleTextOutputProtocol,
	runtime_services: *mut c_void,
	boot_services: *mut BootServices,
	number_of_table_entries: usize,
	configuration_table: *mut c_void,
}

#[repr(C)]
struct SimpleTextOutputProtocol {
	reset: usize,
	output_string:
		extern "efiapi" fn(*mut SimpleTextOutputProtocol, *const u16) -> Status,
}

type HandleProtocol =
	extern "efiapi" fn(Handle, *const Guid, *mut *mut c_void) -> Status;
type AllocatePool = extern "efiapi" fn(u32, usize, *mut *mut c_void) -> Status;
type FreePool = extern "efiapi" fn(*mut c_void) -> Status;
type Stall = extern "efiapi" fn(usize) -> Status;
type LoadImage = extern "efiapi" fn(
	u8,
	Handle,
	*const c_void,
	*const c_void,
	usize,
	*mut Handle,
) -> Status;
type StartImage = extern "efiapi" fn(Handle, *mut usize, *mut *mut u16) -> Status;

#[repr(C)]
struct BootServices {
	header: [u8; 24],
	_raise_tpl: usize,
	_restore_tpl: usize,
	_allocate_pages: usize,
	_free_pages: usize,
	_get_memory_map: usize,
	allocate_pool: AllocatePool,
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
	handle_protocol: HandleProtocol,
	_reserved: usize,
	_register_protocol_notify: usize,
	_locate_handle: usize,
	_locate_device_path: usize,
	_install_configuration_table: usize,
	load_image: LoadImage,
	start_image: StartImage,
	_exit: usize,
	_unload_image: usize,
	_exit_boot_services: usize,
	_get_next_monotonic_count: usize,
	stall: Stall,
	_set_watchdog_timer: usize,
	_connect_controller: usize,
	_disconnect_controller: usize,
	_open_protocol: usize,
	_close_protocol: usize,
	_open_protocol_information: usize,
	_protocols_per_handle: usize,
	_locate_handle_buffer: usize,
	_locate_protocol: usize,
}

#[repr(C)]
struct LoadedImageProtocol {
	revision: u32,
	parent_handle: Handle,
	system_table: *mut SystemTable,
	device_handle: Handle,
	file_path: *mut c_void,
	reserved: *mut c_void,
	load_options_size: u32,
	load_options: *mut c_void,
	image_base: *mut c_void,
	image_size: u64,
	image_code_type: u32,
	image_data_type: u32,
	unload: usize,
}

#[repr(C)]
struct SimpleFileSystemProtocol {
	revision: u64,
	open_volume:
		extern "efiapi" fn(*const SimpleFileSystemProtocol, *mut *mut FileProtocol) -> Status,
}

#[repr(C)]
struct FileProtocol {
	revision: u64,
	open: extern "efiapi" fn(
		*mut FileProtocol,
		*mut *mut FileProtocol,
		*const u16,
		u64,
		u64,
	) -> Status,
	close: extern "efiapi" fn(*mut FileProtocol) -> Status,
	_delete: usize,
	read: extern "efiapi" fn(*mut FileProtocol, *mut usize, *mut c_void) -> Status,
	_write: usize,
	_get_position: usize,
	_set_position: usize,
	get_info:
		extern "efiapi" fn(*mut FileProtocol, *const Guid, *mut usize, *mut c_void) -> Status,
	_set_info: usize,
	_flush: usize,
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

fn halt() -> ! {
	loop {
		core::hint::spin_loop();
	}
}

const LOADED_IMAGE_GUID: Guid = Guid {
	data1: 0x5b1b31a1,
	data2: 0x9562,
	data3: 0x11d2,
	data4: [0x8e, 0x3f, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

const SIMPLE_FILE_SYSTEM_GUID: Guid = Guid {
	data1: 0x964e5b22,
	data2: 0x6459,
	data3: 0x11d2,
	data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

const FILE_INFO_GUID: Guid = Guid {
	data1: 0x09576e92,
	data2: 0x6d3f,
	data3: 0x11d2,
	data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

const INICIO_PATH: [u16; 11] = [
	b'\\' as u16,
	b'i' as u16,
	b'n' as u16,
	b'i' as u16,
	b'c' as u16,
	b'i' as u16,
	b'o' as u16,
	b'.' as u16,
	b's' as u16,
	b'o' as u16,
	0,
];

static WELCOME_MESSAGE: &[u16] = &ascii_to_utf16(b"Bem vindo ao RustOS Beta\r\n\0");

#[no_mangle]
extern "efiapi" fn efi_main(image_handle: Handle, system_table: *mut SystemTable) -> Status {
	unsafe {
		if system_table.is_null() {
			halt();
		}

		let boot_services = (*system_table).boot_services;
		if boot_services.is_null() {
			halt();
		}
		let boot_services = &mut *boot_services;

		let loaded_image = get_loaded_image(boot_services, image_handle);
		let file_system = get_file_system(boot_services, loaded_image);
		let root = open_volume(file_system);
		let inicio_file = open_inicio(root);
		let (inicio_size, info_buffer) = get_inicio_size(boot_services, inicio_file);
		let inicio_buffer = read_inicio(
			boot_services,
			inicio_file,
			root,
			inicio_size,
			info_buffer,
		);

		let mut inicio_handle: Handle = core::ptr::null_mut();
		let status = (boot_services.load_image)(
			0,
			image_handle,
			core::ptr::null(),
			inicio_buffer,
			inicio_size,
			&mut inicio_handle,
		);
		let free_status = (boot_services.free_pool)(inicio_buffer);
		if status != SUCCESS {
			halt();
		}
		if free_status != SUCCESS {
			halt();
		}

		let console = (*system_table).console_out;
		if console.is_null()
			|| ((*console).output_string)(console, WELCOME_MESSAGE.as_ptr()) != SUCCESS
		{
			halt();
		}
		if (boot_services.stall)(2_000_000) != SUCCESS {
			halt();
		}

		let mut exit_data_size = 0;
		let mut exit_data: *mut u16 = core::ptr::null_mut();
		let status =
			(boot_services.start_image)(inicio_handle, &mut exit_data_size, &mut exit_data);
		if status != SUCCESS {
			halt();
		}
	}

	halt()
}

unsafe fn get_loaded_image(
	boot_services: &mut BootServices,
	image_handle: Handle,
) -> &'static LoadedImageProtocol {
	let mut protocol: *mut c_void = core::ptr::null_mut();
	let status = (boot_services.handle_protocol)(
		image_handle,
		&LOADED_IMAGE_GUID,
		&mut protocol,
	);
	if status != SUCCESS || protocol.is_null() {
		halt();
	}
	&*(protocol as *const LoadedImageProtocol)
}

unsafe fn get_file_system(
	boot_services: &mut BootServices,
	loaded_image: &LoadedImageProtocol,
) -> &'static SimpleFileSystemProtocol {
	let mut protocol: *mut c_void = core::ptr::null_mut();
	let status = (boot_services.handle_protocol)(
		loaded_image.device_handle,
		&SIMPLE_FILE_SYSTEM_GUID,
		&mut protocol,
	);
	if status != SUCCESS || protocol.is_null() {
		halt();
	}
	&*(protocol as *const SimpleFileSystemProtocol)
}

unsafe fn open_volume(
	file_system: &SimpleFileSystemProtocol,
) -> *mut FileProtocol {
	let mut root = core::ptr::null_mut();
	let status = (file_system.open_volume)(file_system, &mut root);
	if status != SUCCESS || root.is_null() {
		halt();
	}
	root
}

unsafe fn open_inicio(
	root: *mut FileProtocol,
) -> *mut FileProtocol {
	let mut inicio_file = core::ptr::null_mut();
	let status = ((*root).open)(
		root,
		&mut inicio_file,
		INICIO_PATH.as_ptr(),
		FILE_MODE_READ,
		0,
	);
	if status != SUCCESS || inicio_file.is_null() {
		halt();
	}
	inicio_file
}

unsafe fn get_inicio_size(
	boot_services: &mut BootServices,
	inicio_file: *mut FileProtocol,
) -> (usize, *mut c_void) {
	let mut info_size = 0;
	let status = ((*inicio_file).get_info)(
		inicio_file,
		&FILE_INFO_GUID,
		&mut info_size,
		core::ptr::null_mut(),
	);
	if status != BUFFER_TOO_SMALL || info_size < 16 {
		halt();
	}

	let mut info_buffer: *mut c_void = core::ptr::null_mut();
	let status = (boot_services.allocate_pool)(LOADER_DATA, info_size, &mut info_buffer);
	if status != SUCCESS || info_buffer.is_null() {
		halt();
	}

	let status = ((*inicio_file).get_info)(
		inicio_file,
		&FILE_INFO_GUID,
		&mut info_size,
		info_buffer,
	);
	if status != SUCCESS {
		halt();
	}

	let bytes = core::slice::from_raw_parts(info_buffer as *const u8, info_size);
	let mut size_bytes = [0; 8];
	size_bytes.copy_from_slice(&bytes[8..16]);
	let inicio_size_u64 = u64::from_le_bytes(size_bytes);
	if inicio_size_u64 == 0 || inicio_size_u64 > usize::MAX as u64 {
		halt();
	}
	(inicio_size_u64 as usize, info_buffer)
}

unsafe fn read_inicio(
	boot_services: &mut BootServices,
	inicio_file: *mut FileProtocol,
	root: *mut FileProtocol,
	inicio_size: usize,
	info_buffer: *mut c_void,
) -> *mut c_void {
	let free_status = (boot_services.free_pool)(info_buffer);
	if free_status != SUCCESS {
		halt();
	}

	let mut inicio_buffer: *mut c_void = core::ptr::null_mut();
	let status = (boot_services.allocate_pool)(LOADER_DATA, inicio_size, &mut inicio_buffer);
	if status != SUCCESS || inicio_buffer.is_null() {
		halt();
	}

	let mut bytes_read = inicio_size;
	let status = ((*inicio_file).read)(inicio_file, &mut bytes_read, inicio_buffer);
	let inicio_close_status = ((*inicio_file).close)(inicio_file);
	let root_close_status = ((*root).close)(root);
	if status != SUCCESS || bytes_read != inicio_size {
		halt();
	}
	if inicio_close_status != SUCCESS || root_close_status != SUCCESS {
		halt();
	}
	inicio_buffer
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	halt()
}

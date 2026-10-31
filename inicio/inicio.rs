// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.7.2026
// Funcao: Configura o framebuffer, carrega o nucleo ELF da ISO e transfere o controle.
#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

type Handle = *mut c_void;
type Status = usize;

const SUCCESS: Status = 0;
const ALLOCATE_ANY_PAGES: u32 = 0;
const ALLOCATE_ADDRESS: u32 = 2;
const LOADER_CODE: u32 = 1;
const LOADER_DATA: u32 = 2;
const PAGE_SIZE: usize = 4096;
const MEMORY_DESCRIPTOR_VERSION: u32 = 1;
const MEMORY_DESCRIPTOR_MIN_SIZE: usize = 40;
const ISO_BLOCK_SIZE: usize = 2048;
const MAX_KERNEL_SIZE: usize = 64 * 1024 * 1024;
const BUFFER_TOO_SMALL: Status = (1usize << (usize::BITS - 1)) | 5;
const INVALID_PARAMETER: Status = (1usize << (usize::BITS - 1)) | 2;
const PIXEL_RGB_RESERVED_8: u32 = 0;
const PIXEL_BGR_RESERVED_8: u32 = 1;
const MAX_FIRMWARE_TABLES: usize = 32;
const TABLE_KIND_OTHER: u32 = 0;
const TABLE_KIND_ACPI_10: u32 = 1;
const TABLE_KIND_ACPI_20: u32 = 2;
const TABLE_KIND_SMBIOS: u32 = 3;
const TABLE_KIND_SMBIOS3: u32 = 4;
const RNG_SEED_SIZE: usize = 32;
const RNG_SEED_NO_CPU_SUPPORT: u32 = 1;
const RNG_SEED_CPU_NO_RESPONSE: u32 = 2;
const RNG_SEED_NO_SOURCE: u32 = 3;
const RNG_SEED_TPM_NO_RESPONSE: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
	data1: u32,
	data2: u16,
	data3: u16,
	data4: [u8; 8],
}

#[repr(C)]
struct EfiTableHeader {
	signature: u64,
	revision: u32,
	header_size: u32,
	crc32: u32,
	reserved: u32,
}

#[repr(C)]
struct SystemTable {
	header: EfiTableHeader,
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

#[repr(C)]
struct EfiConfigurationTable {
	vendor_guid: Guid,
	vendor_table: *const c_void,
}

type AllocatePages =
	extern "efiapi" fn(u32, u32, usize, *mut u64) -> Status;
type FreePages = extern "efiapi" fn(u64, usize) -> Status;
type AllocatePool = extern "efiapi" fn(u32, usize, *mut *mut c_void) -> Status;
type FreePool = extern "efiapi" fn(*mut c_void) -> Status;
type GetMemoryMap = extern "efiapi" fn(
	*mut usize,
	*mut c_void,
	*mut usize,
	*mut usize,
	*mut u32,
) -> Status;
type ExitBootServices = extern "efiapi" fn(Handle, usize) -> Status;
type HandleProtocol =
	extern "efiapi" fn(Handle, *const Guid, *mut *mut c_void) -> Status;
type LocateHandleBuffer = extern "efiapi" fn(
	u32,
	*const Guid,
	*mut c_void,
	*mut usize,
	*mut *mut Handle,
) -> Status;
type LocateProtocol =
	extern "efiapi" fn(*const Guid, *mut c_void, *mut *mut c_void) -> Status;
type GetRng = extern "efiapi" fn(*mut RngProtocol, *const Guid, usize, *mut u8) -> Status;
type Tcg2SubmitCommand = extern "efiapi" fn(
	*mut Tcg2Protocol,
	u32,
	*const u8,
	u32,
	*mut u8,
) -> Status;

#[repr(C)]
struct BootServices {
	header: [u8; 24],
	_raise_tpl: usize,
	_restore_tpl: usize,
	allocate_pages: AllocatePages,
	free_pages: FreePages,
	get_memory_map: GetMemoryMap,
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
	_load_image: usize,
	_start_image: usize,
	_exit: usize,
	_unload_image: usize,
	exit_boot_services: ExitBootServices,
	_get_next_monotonic_count: usize,
	_stall: usize,
	_set_watchdog_timer: usize,
	_connect_controller: usize,
	_disconnect_controller: usize,
	_open_protocol: usize,
	_close_protocol: usize,
	_open_protocol_information: usize,
	_protocols_per_handle: usize,
	locate_handle_buffer: LocateHandleBuffer,
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
	blt: usize,
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
struct BlockIoMedia {
	media_id: u32,
	removable_media: u8,
	media_present: u8,
	logical_partition: u8,
	read_only: u8,
	write_caching: u8,
	block_size: u32,
	io_align: u32,
	last_block: u64,
	lowest_aligned_lba: u64,
	logical_blocks_per_physical_block: u32,
	optimal_transfer_length_granularity: u32,
}

type ReadBlocks =
	extern "efiapi" fn(*mut BlockIoProtocol, u32, u64, usize, *mut c_void) -> Status;

#[repr(C)]
struct BlockIoProtocol {
	revision: u64,
	media: *mut BlockIoMedia,
	_reset: usize,
	read_blocks: ReadBlocks,
	_write_blocks: usize,
	_flush_blocks: usize,
}

#[repr(C)]
struct BootInfo {
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
	firmware_tables: [FirmwareTablePointer; MAX_FIRMWARE_TABLES],
	firmware_table_count: usize,
	rng_seed: [u8; RNG_SEED_SIZE],
	rng_seed_status: u32,
}

#[repr(C)]
struct RngProtocol {
	_get_info: usize,
	get_rng: GetRng,
}

#[repr(C)]
struct Tcg2Protocol {
	_get_capability: usize,
	_get_event_log: usize,
	_hash_log_extend_event: usize,
	submit_command: Tcg2SubmitCommand,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FirmwareTablePointer {
	address: u64,
	kind: u32,
}

struct MemoryMapInfo {
	buffer: *const u8,
	size: usize,
	descriptor_size: usize,
}

#[derive(Clone, Copy)]
struct IsoFile {
	extent: u32,
	size: u32,
}

const GRAPHICS_OUTPUT_GUID: Guid = Guid {
	data1: 0x9042a9de,
	data2: 0x23dc,
	data3: 0x4a38,
	data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

const BLOCK_IO_GUID: Guid = Guid {
	data1: 0x964e5b21,
	data2: 0x6459,
	data3: 0x11d2,
	data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

const ACPI_10_TABLE_GUID: Guid = Guid {
	data1: 0xeb9d_2d30,
	data2: 0x2d88,
	data3: 0x11d3,
	data4: [0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
};

const ACPI_20_TABLE_GUID: Guid = Guid {
	data1: 0x8868_e871,
	data2: 0xe4f1,
	data3: 0x11d3,
	data4: [0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
};

const SMBIOS_TABLE_GUID: Guid = Guid {
	data1: 0xeb9d_2d31,
	data2: 0x2d88,
	data3: 0x11d3,
	data4: [0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
};

const SMBIOS3_TABLE_GUID: Guid = Guid {
	data1: 0xf2fd_1544,
	data2: 0x9794,
	data3: 0x4a2c,
	data4: [0x99, 0x2e, 0xe5, 0xbb, 0xcf, 0x20, 0xe3, 0x94],
};

const RNG_PROTOCOL_GUID: Guid = Guid {
	data1: 0x3152_bca5,
	data2: 0xeade,
	data3: 0x433d,
	data4: [0x86, 0x2e, 0xc0, 0x1c, 0xdc, 0x29, 0x1f, 0x44],
};

const TCG2_PROTOCOL_GUID: Guid = Guid {
	data1: 0x607f_766c,
	data2: 0x7455,
	data3: 0x42be,
	data4: [0x93, 0x0b, 0xe4, 0xd7, 0x6d, 0xb2, 0x72, 0x0f],
};

#[no_mangle]
extern "efiapi" fn efi_main(image_handle: Handle, system_table: *mut SystemTable) -> Status {
	if system_table.is_null() {
		halt();
	}

	unsafe {
		let acpi_rsdp = find_acpi_rsdp(&*system_table);
		let efi_revision = (*system_table).header.revision;
		let boot_services = (*system_table).boot_services;
		if boot_services.is_null() {
			halt();
		}
		let boot_services = &mut *boot_services;
		let mut boot_info = configure_frame_buffer(boot_services);
		boot_info.acpi_rsdp = acpi_rsdp;
		boot_info.efi_revision = efi_revision;
		copy_firmware_table_pointers(&*system_table, &mut boot_info);
	match obter_seed(boot_services) {
		Ok(seed) => {
			boot_info.rng_seed = seed;
			boot_info.rng_seed_status = 0;
		}
		Err(status) => {
			boot_info.rng_seed = [0; RNG_SEED_SIZE];
			boot_info.rng_seed_status = status;
		}
	}
		let (block_io, kernel_file) = find_kernel_file(boot_services);
		let (kernel_buffer, kernel_size, kernel_pages) =
			read_kernel_file(boot_services, block_io, kernel_file);
		let entry = load_elf_kernel(boot_services, kernel_buffer, kernel_size);

		if (boot_services.free_pages)(kernel_buffer as u64, kernel_pages) != SUCCESS {
			halt();
		}

		let memory_map = exit_boot_services(boot_services, image_handle);
		boot_info.memory_map = memory_map.buffer;
		boot_info.memory_map_size = memory_map.size;
		boot_info.memory_descriptor_size = memory_map.descriptor_size;
		core::arch::asm!("cli", options(nomem, nostack));
		let entry: extern "sysv64" fn(*mut BootInfo) -> ! =
			core::mem::transmute(entry);
		entry(&mut boot_info);
	}
}

unsafe fn find_acpi_rsdp(system_table: &SystemTable) -> *const u8 {
	if system_table.configuration_table.is_null() || system_table.number_of_table_entries == 0 {
		return core::ptr::null();
	}
	let tables = core::slice::from_raw_parts(
		system_table.configuration_table.cast::<EfiConfigurationTable>(),
		system_table.number_of_table_entries,
	);
	for wanted in [ACPI_20_TABLE_GUID, ACPI_10_TABLE_GUID] {
		if let Some(table) = tables.iter().find(|table| {
			table.vendor_guid.data1 == wanted.data1
				&& table.vendor_guid.data2 == wanted.data2
				&& table.vendor_guid.data3 == wanted.data3
				&& table.vendor_guid.data4 == wanted.data4
		}) {
			return table.vendor_table.cast();
		}
	}
	core::ptr::null()
}

unsafe fn obter_seed(boot_services: &mut BootServices) -> Result<[u8; RNG_SEED_SIZE], u32> {
	let mut protocol = core::ptr::null_mut();
	if (boot_services.locate_protocol)(
		&RNG_PROTOCOL_GUID,
		core::ptr::null_mut(),
		&mut protocol,
	) == SUCCESS
		&& !protocol.is_null()
	{
		let rng = &mut *(protocol as *mut RngProtocol);
		let mut seed = [0; RNG_SEED_SIZE];
		if (rng.get_rng)(rng, core::ptr::null(), seed.len(), seed.as_mut_ptr()) == SUCCESS
			&& seed.iter().any(|byte| *byte != 0)
		{
			return Ok(seed);
		}
	}
	match obter_seed_cpu() {
		Ok(seed) => Ok(seed),
		Err(cpu_status) => match obter_seed_tpm(boot_services) {
			Ok(seed) => Ok(seed),
			Err(RNG_SEED_TPM_NO_RESPONSE) => Err(RNG_SEED_TPM_NO_RESPONSE),
			Err(_) if cpu_status == RNG_SEED_NO_CPU_SUPPORT => Err(RNG_SEED_NO_SOURCE),
			Err(_) => Err(cpu_status),
		},
	}
}

unsafe fn obter_seed_tpm(boot_services: &mut BootServices) -> Result<[u8; RNG_SEED_SIZE], u32> {
	let mut protocol = core::ptr::null_mut();
	if (boot_services.locate_protocol)(
		&TCG2_PROTOCOL_GUID,
		core::ptr::null_mut(),
		&mut protocol,
	) != SUCCESS
		|| protocol.is_null()
	{
		return Err(RNG_SEED_NO_SOURCE);
	}

	let tcg2 = &mut *(protocol as *mut Tcg2Protocol);
	let mut seed = [0; RNG_SEED_SIZE];
	let mut written = 0;
	for _ in 0..4 {
		if written == seed.len() {
			break;
		}
		let requested = (seed.len() - written) as u16;
		let command = [
			0x80,
			0x01,
			0x00,
			0x00,
			0x00,
			0x0c,
			0x00,
			0x00,
			0x01,
			0x7b,
			(requested >> 8) as u8,
			requested as u8,
		];
		let mut response = [0u8; 64];
		if (tcg2.submit_command)(
			tcg2,
			command.len() as u32,
			command.as_ptr(),
			response.len() as u32,
			response.as_mut_ptr(),
		) != SUCCESS
		{
			apagar_seed(&mut seed);
			return Err(RNG_SEED_TPM_NO_RESPONSE);
		}

		let response_size = u32::from_be_bytes([
			response[2], response[3], response[4], response[5],
		]) as usize;
		let response_code = u32::from_be_bytes([
			response[6], response[7], response[8], response[9],
		]);
		let received = u16::from_be_bytes([response[10], response[11]]) as usize;
		if response[0..2] != [0x80, 0x01]
			|| response_code != 0
			|| !(12..=response.len()).contains(&response_size)
			|| received == 0
			|| received > requested as usize
			|| response_size != 12 + received
		{
			apagar_seed(&mut seed);
			return Err(RNG_SEED_TPM_NO_RESPONSE);
		}
		seed[written..written + received].copy_from_slice(&response[12..12 + received]);
		written += received;
	}

	if written == seed.len() && seed.iter().any(|byte| *byte != 0) {
		Ok(seed)
	} else {
		apagar_seed(&mut seed);
		Err(RNG_SEED_TPM_NO_RESPONSE)
	}
}

fn apagar_seed(seed: &mut [u8; RNG_SEED_SIZE]) {
	for byte in seed {
		unsafe { core::ptr::write_volatile(byte, 0) };
	}
}

unsafe fn obter_seed_cpu() -> Result<[u8; RNG_SEED_SIZE], u32> {
	let maximum_leaf = core::arch::x86_64::__cpuid(0).eax;
	let use_rdseed = maximum_leaf >= 7
		&& core::arch::x86_64::__cpuid_count(7, 0).ebx & (1 << 18) != 0;
	let use_rdrand = maximum_leaf >= 1 && core::arch::x86_64::__cpuid(1).ecx & (1 << 30) != 0;
	if !use_rdseed && !use_rdrand {
		return Err(RNG_SEED_NO_CPU_SUPPORT);
	}

	let mut seed = [0; RNG_SEED_SIZE];
	if use_rdseed && preencher_seed_cpu(&mut seed, true) {
		return Ok(seed);
	}
	if use_rdrand && preencher_seed_cpu(&mut seed, false) {
		return Ok(seed);
	}
	Err(RNG_SEED_CPU_NO_RESPONSE)
}

unsafe fn preencher_seed_cpu(seed: &mut [u8; RNG_SEED_SIZE], use_rdseed: bool) -> bool {
	for chunk in seed.chunks_exact_mut(core::mem::size_of::<u64>()) {
		let mut value = None;
		for _ in 0..10_000 {
			value = if use_rdseed {
				try_rdseed()
			} else {
				try_rdrand()
			};
			if value.is_some() {
				break;
			}
			core::hint::spin_loop();
		}
		let Some(value) = value else {
			for byte in seed.iter_mut() {
				core::ptr::write_volatile(byte, 0);
			}
			return false;
		};
		chunk.copy_from_slice(&value.to_le_bytes());
	}
	seed.iter().any(|byte| *byte != 0)
}

unsafe fn try_rdseed() -> Option<u64> {
	let value: u64;
	let success: u8;
	core::arch::asm!(
		"rdseed {value}",
		"setc {success}",
		value = out(reg) value,
		success = out(reg_byte) success,
		options(nomem, nostack)
	);
	(success != 0).then_some(value)
}

unsafe fn try_rdrand() -> Option<u64> {
	let value: u64;
	let success: u8;
	core::arch::asm!(
		"rdrand {value}",
		"setc {success}",
		value = out(reg) value,
		success = out(reg_byte) success,
		options(nomem, nostack)
	);
	(success != 0).then_some(value)
}

unsafe fn copy_firmware_table_pointers(system_table: &SystemTable, boot_info: &mut BootInfo) {
	if system_table.configuration_table.is_null() || system_table.number_of_table_entries == 0 {
		return;
	}
	let tables = core::slice::from_raw_parts(
		system_table.configuration_table.cast::<EfiConfigurationTable>(),
		system_table.number_of_table_entries,
	);
	for table in tables {
		if boot_info.firmware_table_count == MAX_FIRMWARE_TABLES {
			break;
		}
		if table.vendor_table.is_null() {
			continue;
		}
		let kind = if guid_matches(&table.vendor_guid, &ACPI_10_TABLE_GUID) {
			TABLE_KIND_ACPI_10
		} else if guid_matches(&table.vendor_guid, &ACPI_20_TABLE_GUID) {
			TABLE_KIND_ACPI_20
		} else if guid_matches(&table.vendor_guid, &SMBIOS_TABLE_GUID) {
			TABLE_KIND_SMBIOS
		} else if guid_matches(&table.vendor_guid, &SMBIOS3_TABLE_GUID) {
			TABLE_KIND_SMBIOS3
		} else {
			TABLE_KIND_OTHER
		};
		let index = boot_info.firmware_table_count;
		boot_info.firmware_tables[index] = FirmwareTablePointer {
			address: table.vendor_table as u64,
			kind,
		};
		boot_info.firmware_table_count += 1;
	}
}

fn guid_matches(left: &Guid, right: &Guid) -> bool {
	left.data1 == right.data1
		&& left.data2 == right.data2
		&& left.data3 == right.data3
		&& left.data4 == right.data4
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

unsafe fn exit_boot_services(
	boot_services: &mut BootServices,
	image_handle: Handle,
) -> MemoryMapInfo {
	let mut map_buffer: *mut c_void = core::ptr::null_mut();
	let mut map_capacity = 64 * 1024;
	if (boot_services.allocate_pool)(LOADER_DATA, map_capacity, &mut map_buffer) != SUCCESS
		|| map_buffer.is_null()
	{
		halt();
	}

	loop {
		let mut map_size = map_capacity;
		let mut map_key = 0;
		let mut descriptor_size = 0;
		let mut descriptor_version = 0;
		let status = (boot_services.get_memory_map)(
			&mut map_size,
			map_buffer,
			&mut map_key,
			&mut descriptor_size,
			&mut descriptor_version,
		);
		if status == BUFFER_TOO_SMALL {
			if (boot_services.free_pool)(map_buffer) != SUCCESS {
				halt();
			}
			let extra_size = descriptor_size.checked_mul(8).unwrap_or_else(|| halt());
			map_capacity = map_size
				.checked_add(extra_size)
				.unwrap_or_else(|| halt());
			if (boot_services.allocate_pool)(
				LOADER_DATA,
				map_capacity,
				&mut map_buffer,
			) != SUCCESS
				|| map_buffer.is_null()
			{
				halt();
			}
			continue;
		}
		if status != SUCCESS {
			halt();
		}
		if descriptor_version != MEMORY_DESCRIPTOR_VERSION
			|| descriptor_size < MEMORY_DESCRIPTOR_MIN_SIZE
		{
			halt();
		}
		let status = (boot_services.exit_boot_services)(image_handle, map_key);
		if status == SUCCESS {
			return MemoryMapInfo {
				buffer: map_buffer.cast(),
				size: map_size,
				descriptor_size,
			};
		}
		if status != INVALID_PARAMETER {
			halt();
		}
	}
}

unsafe fn configure_frame_buffer(boot_services: &mut BootServices) -> BootInfo {
	let mut protocol: *mut c_void = core::ptr::null_mut();
	if (boot_services.locate_protocol)(
		&GRAPHICS_OUTPUT_GUID,
		core::ptr::null_mut(),
		&mut protocol,
	) != SUCCESS
		|| protocol.is_null()
	{
		halt();
	}

	let graphics = protocol as *mut GraphicsOutputProtocol;
	if (*graphics).mode.is_null() {
		halt();
	}

	let mut selected_mode = None;
	for mode_number in 0..(*(*graphics).mode).max_mode {
		let mut info_size = 0;
		let mut info = core::ptr::null_mut();
		if ((*graphics).query_mode)(graphics, mode_number, &mut info_size, &mut info) != SUCCESS
			|| info.is_null()
		{
			halt();
		}
		let matches =
			(*info).horizontal_resolution == 640 && (*info).vertical_resolution == 480;
		if (boot_services.free_pool)(info as *mut c_void) != SUCCESS {
			halt();
		}
		if matches {
			selected_mode = Some(mode_number);
			break;
		}
	}

	let Some(selected_mode) = selected_mode else {
		halt();
	};
	if ((*graphics).set_mode)(graphics, selected_mode) != SUCCESS {
		halt();
	}

	let mode = &*(*graphics).mode;
	if mode.info.is_null() {
		halt();
	}
	let info = &*mode.info;
	let width = info.horizontal_resolution as usize;
	let height = info.vertical_resolution as usize;
	let stride = info.pixels_per_scan_line as usize;
	let Some(frame_buffer_size) = stride.checked_mul(height).and_then(|n| n.checked_mul(4))
	else {
		halt();
	};
	if !matches!(info.pixel_format, PIXEL_RGB_RESERVED_8 | PIXEL_BGR_RESERVED_8)
		|| stride < width
		|| mode.frame_buffer_base == 0
		|| mode.frame_buffer_size < frame_buffer_size
	{
		halt();
	}

	let frame_buffer = mode.frame_buffer_base as *mut u32;
	for pixel in 0..frame_buffer_size / 4 {
		core::ptr::write_volatile(frame_buffer.add(pixel), 0);
	}
	BootInfo {
		frame_buffer: mode.frame_buffer_base as *mut u32,
		width,
		height,
		pixels_per_scan_line: stride,
		pixel_format: info.pixel_format,
		memory_map: core::ptr::null(),
		memory_map_size: 0,
		memory_descriptor_size: 0,
		acpi_rsdp: core::ptr::null(),
		efi_revision: 0,
		firmware_tables: [
			FirmwareTablePointer {
				address: 0,
				kind: TABLE_KIND_OTHER,
			};
			MAX_FIRMWARE_TABLES
		],
		firmware_table_count: 0,
		rng_seed: [0; RNG_SEED_SIZE],
		rng_seed_status: RNG_SEED_NO_CPU_SUPPORT,
	}
}

unsafe fn find_kernel_file(
	boot_services: &mut BootServices,
) -> (*mut BlockIoProtocol, IsoFile) {
	let mut handle_count = 0;
	let mut handles: *mut Handle = core::ptr::null_mut();
	let status = (boot_services.locate_handle_buffer)(
		2,
		&BLOCK_IO_GUID,
		core::ptr::null_mut(),
		&mut handle_count,
		&mut handles,
	);
	if status != SUCCESS || handles.is_null() || handle_count == 0 {
		halt();
	}

	let mut scratch_address = 0;
	if (boot_services.allocate_pages)(
		ALLOCATE_ANY_PAGES,
		LOADER_DATA,
		1,
		&mut scratch_address,
	) != SUCCESS
		|| scratch_address == 0
	{
		let _ = (boot_services.free_pool)(handles as *mut c_void);
		halt();
	}
	let scratch = scratch_address as *mut u8;
	let mut found = None;

	for handle in core::slice::from_raw_parts(handles, handle_count) {
		let mut protocol: *mut c_void = core::ptr::null_mut();
		if (boot_services.handle_protocol)(
			*handle,
			&BLOCK_IO_GUID,
			&mut protocol,
		) != SUCCESS
			|| protocol.is_null()
		{
			continue;
		}
		let block_io = protocol as *mut BlockIoProtocol;
		if (*block_io).media.is_null() {
			continue;
		}
		let media = &*(*block_io).media;
		if media.media_present == 0 || media.block_size as usize != ISO_BLOCK_SIZE {
			continue;
		}
		if let Some(file) = find_iso_kernel(block_io, scratch) {
			found = Some((block_io, file));
			break;
		}
	}

	if (boot_services.free_pages)(scratch_address, 1) != SUCCESS
		|| (boot_services.free_pool)(handles as *mut c_void) != SUCCESS
	{
		halt();
	}
	found.unwrap_or_else(|| halt())
}

unsafe fn find_iso_kernel(
	block_io: *mut BlockIoProtocol,
	scratch: *mut u8,
) -> Option<IsoFile> {
	if !read_iso_block(block_io, 16, scratch) {
		return None;
	}
	let volume = core::slice::from_raw_parts(scratch, ISO_BLOCK_SIZE);
	if volume[0] != 1
		|| &volume[1..6] != b"CD001"
		|| volume[6] != 1
	{
		return None;
	}
	let root_record_size = volume[156] as usize;
	if root_record_size < 34 || 156 + root_record_size > ISO_BLOCK_SIZE {
		return None;
	}
	let root_extent = read_u32_le(volume, 158)?;
	let root_size = read_u32_le(volume, 166)?;
	let nucleo = find_directory_entry(block_io, scratch, root_extent, root_size, b"NUCLEO")?;
	if nucleo.directory == 0 {
		return None;
	}
	let rust = find_directory_entry(
		block_io,
		scratch,
		nucleo.file.extent,
		nucleo.file.size,
		b"RUST",
	)?;
	if rust.directory != 0 || rust.file.size == 0 {
		return None;
	}
	Some(rust.file)
}

#[derive(Clone, Copy)]
struct IsoEntry {
	file: IsoFile,
	directory: u8,
}

unsafe fn find_directory_entry(
	block_io: *mut BlockIoProtocol,
	scratch: *mut u8,
	extent: u32,
	directory_size: u32,
	wanted: &[u8],
) -> Option<IsoEntry> {
	let mut consumed = 0usize;
	while consumed < directory_size as usize {
		let block = extent.checked_add((consumed / ISO_BLOCK_SIZE) as u32)?;
		if !read_iso_block(block_io, block as u64, scratch) {
			return None;
		}
		let sector = core::slice::from_raw_parts(scratch, ISO_BLOCK_SIZE);
		let valid = (directory_size as usize - consumed).min(ISO_BLOCK_SIZE);
		let mut offset = 0usize;
		while offset < valid {
			let record_size = sector[offset] as usize;
			if record_size == 0 {
				break;
			}
			if record_size < 34 || offset + record_size > valid {
				return None;
			}
			let name_length = sector[offset + 32] as usize;
			if 33 + name_length > record_size {
				return None;
			}
			let name = &sector[offset + 33..offset + 33 + name_length];
			if iso_name_matches(name, wanted) {
				return Some(IsoEntry {
					file: IsoFile {
						extent: read_u32_le(sector, offset + 2)?,
						size: read_u32_le(sector, offset + 10)?,
					},
					directory: sector[offset + 25] & 2,
				});
			}
			offset += record_size;
		}
		consumed = consumed.checked_add(valid)?;
	}
	None
}

fn iso_name_matches(name: &[u8], wanted: &[u8]) -> bool {
	let mut end = name.iter().position(|byte| *byte == b';').unwrap_or(name.len());
	if end > 0 && name[end - 1] == b'.' {
		end -= 1;
	}
	end == wanted.len()
		&& name[..end]
			.iter()
			.zip(wanted)
			.all(|(left, right)| left.to_ascii_uppercase() == *right)
}

unsafe fn read_iso_block(
	block_io: *mut BlockIoProtocol,
	block: u64,
	buffer: *mut u8,
) -> bool {
	let media = (*block_io).media;
	!media.is_null()
		&& ((*block_io).read_blocks)(
			block_io,
			(*media).media_id,
			block,
			ISO_BLOCK_SIZE,
			buffer as *mut c_void,
		) == SUCCESS
}

unsafe fn read_kernel_file(
	boot_services: &mut BootServices,
	block_io: *mut BlockIoProtocol,
	file: IsoFile,
) -> (*mut u8, usize, usize) {
	let size = file.size as usize;
	if size > MAX_KERNEL_SIZE {
		halt();
	}
	let Some(padded_size) = size
		.checked_add(ISO_BLOCK_SIZE - 1)
		.map(|n| n / ISO_BLOCK_SIZE * ISO_BLOCK_SIZE)
	else {
		halt();
	};
	let Some(pages) = padded_size.checked_add(PAGE_SIZE - 1).map(|n| n / PAGE_SIZE) else {
		halt();
	};
	let mut address = 0;
	if (boot_services.allocate_pages)(
		ALLOCATE_ANY_PAGES,
		LOADER_DATA,
		pages,
		&mut address,
	) != SUCCESS
		|| address == 0
	{
		halt();
	}
	if ((*block_io).read_blocks)(
		block_io,
		(*(*block_io).media).media_id,
		file.extent as u64,
		padded_size,
		address as *mut c_void,
	) != SUCCESS
	{
		let _ = (boot_services.free_pages)(address, pages);
		halt();
	}
	(address as *mut u8, size, pages)
}

unsafe fn load_elf_kernel(
	boot_services: &mut BootServices,
	kernel: *const u8,
	kernel_size: usize,
) -> u64 {
	let elf = core::slice::from_raw_parts(kernel, kernel_size);
	if elf.len() < 64
		|| &elf[0..4] != b"\x7fELF"
		|| elf[4] != 2
		|| elf[5] != 1
		|| read_u16_le(elf, 16) != Some(2)
		|| read_u16_le(elf, 18) != Some(62)
		|| read_u32_le(elf, 20) != Some(1)
	{
		halt();
	}
	let entry = read_u64_le(elf, 24).unwrap_or_else(|| halt());
	let ph_offset = read_u64_le(elf, 32).unwrap_or_else(|| halt());
	let ph_entry_size = read_u16_le(elf, 54).unwrap_or_else(|| halt()) as usize;
	let ph_count = read_u16_le(elf, 56).unwrap_or_else(|| halt()) as usize;
	if ph_entry_size < 56 || ph_count == 0 {
		halt();
	}

	let mut loaded_entry = false;
	for index in 0..ph_count {
		let Some(offset) = (ph_offset as usize)
			.checked_add(index.checked_mul(ph_entry_size).unwrap_or_else(|| halt()))
		else {
			halt();
		};
		let Some(header_end) = offset.checked_add(ph_entry_size) else {
			halt();
		};
		if header_end > elf.len() {
			halt();
		}
		if read_u32_le(elf, offset) != Some(1) {
			continue;
		}

		let file_offset = read_u64_le(elf, offset + 8).unwrap_or_else(|| halt());
		let virtual_address = read_u64_le(elf, offset + 16).unwrap_or_else(|| halt());
		let physical_address = read_u64_le(elf, offset + 24).unwrap_or_else(|| halt());
		let file_size = read_u64_le(elf, offset + 32).unwrap_or_else(|| halt());
		let memory_size = read_u64_le(elf, offset + 40).unwrap_or_else(|| halt());
		if virtual_address != physical_address
			|| physical_address < 0x10_0000
			|| physical_address as usize % PAGE_SIZE != 0
			|| memory_size < file_size
			|| memory_size == 0
			|| memory_size > MAX_KERNEL_SIZE as u64
		{
			halt();
		}
		let Some(file_end) = file_offset.checked_add(file_size) else {
			halt();
		};
		let Some(memory_end) = physical_address.checked_add(memory_size) else {
			halt();
		};
		if file_end > kernel_size as u64 {
			halt();
		}
		let Some(pages) = (memory_size as usize)
			.checked_add(PAGE_SIZE - 1)
			.map(|n| n / PAGE_SIZE)
		else {
			halt();
		};
		let mut address = physical_address;
		if (boot_services.allocate_pages)(
			ALLOCATE_ADDRESS,
			LOADER_CODE,
			pages,
			&mut address,
		) != SUCCESS
			|| address != physical_address
		{
			halt();
		}
		core::ptr::copy_nonoverlapping(
			kernel.add(file_offset as usize),
			physical_address as *mut u8,
			file_size as usize,
		);
		core::ptr::write_bytes(
			(physical_address + file_size) as *mut u8,
			0,
			(memory_size - file_size) as usize,
		);
		let flags = read_u32_le(elf, offset + 4).unwrap_or_else(|| halt());
		if flags & 1 != 0 && entry >= physical_address && entry < memory_end {
			loaded_entry = true;
		}
	}
	if !loaded_entry {
		halt();
	}
	entry
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
	let value = bytes.get(offset..offset.checked_add(2)?)?;
	Some(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
	let value = bytes.get(offset..offset.checked_add(4)?)?;
	Some(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u64_le(bytes: &[u8], offset: usize) -> Option<u64> {
	let value = bytes.get(offset..offset.checked_add(8)?)?;
	Some(u64::from_le_bytes([
		value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
	]))
}

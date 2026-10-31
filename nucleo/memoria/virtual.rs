// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.6.2026
// Funcao: Configura o mapeamento virtual identidade e as protecoes de paginas do nucleo.
use core::ptr;

use super::BootInfo;

const PAGE_SIZE: u64 = 4096;
const PAGE_MASK: u64 = 0x000f_ffff_ffff_f000;
const PRESENT: u64 = 1;
const WRITABLE: u64 = 1 << 1;
const USER: u64 = 1 << 2;
const WRITE_THROUGH: u64 = 1 << 3;
const CACHE_DISABLE: u64 = 1 << 4;
const ACCESSED: u64 = 1 << 5;
const DIRTY: u64 = 1 << 6;
const LARGE_PAGE: u64 = 1 << 7;
const GLOBAL: u64 = 1 << 8;
const LARGE_PAGE_PAT: u64 = 1 << 12;
const NO_EXECUTE: u64 = 1 << 63;
const CR4_LA57: u64 = 1 << 12;
const EFER_MSR: u32 = 0xc000_0080;
const EFER_NXE: u64 = 1 << 11;
const CR0_WRITE_PROTECT: u64 = 1 << 16;
const TWO_MIB: u64 = 0x20_0000;
const MAX_PHYSICAL_ADDRESS: u64 = 64 * 1024 * 1024 * 1024;
const PAGE_DIRECTORIES: usize = (MAX_PHYSICAL_ADDRESS / (512 * TWO_MIB)) as usize;
const MAX_PAGE_TABLES: usize = 256;

#[repr(C, align(4096))]
struct PageTablePool([[u64; 512]; MAX_PAGE_TABLES]);

static mut PAGE_TABLE_POOL: PageTablePool = PageTablePool([[0; 512]; MAX_PAGE_TABLES]);
static mut NEXT_PAGE_TABLE: usize = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroVirtual {
	UnsupportedCpu,
	InvalidRange,
	UnmappedPage,
	ConflictingMapping,
	PageTablePoolExhausted,
}

#[derive(Clone, Copy)]
struct Permissions {
	writable: bool,
	user: bool,
	no_execute: bool,
}

unsafe extern "C" {
	static __text_start: u8;
	static __text_end: u8;
	static __rodata_start: u8;
	static __rodata_end: u8;
	static __data_start: u8;
	static __data_end: u8;
}

pub(super) fn inicializar(boot_info: &BootInfo) -> Result<(), ErroVirtual> {
	let extended_leaf = core::arch::x86_64::__cpuid(0x8000_0000).eax;
	if extended_leaf < 0x8000_0001
		|| core::arch::x86_64::__cpuid(0x8000_0001).edx & (1 << 20) == 0
	{
		return Err(ErroVirtual::UnsupportedCpu);
	}

	let cr4: u64;
	let cr0: u64;
	unsafe {
		core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
		core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
	}
	let five_level_paging = cr4 & CR4_LA57 != 0;

	let frame_buffer_size = boot_info
		.pixels_per_scan_line
		.checked_mul(boot_info.height)
		.and_then(|pixels| pixels.checked_mul(core::mem::size_of::<u32>()))
		.ok_or(ErroVirtual::InvalidRange)?;
	validar_intervalo(boot_info as *const BootInfo as u64, core::mem::size_of::<BootInfo>())?;
	validar_intervalo(boot_info.memory_map as u64, boot_info.memory_map_size)?;
	validar_intervalo(boot_info.frame_buffer as u64, frame_buffer_size)?;
	let stack_pointer: u64;
	unsafe {
		core::arch::asm!("mov {}, rsp", out(reg) stack_pointer, options(nomem, nostack, preserves_flags));
	}
	validar_intervalo(stack_pointer, 1)?;

	let mut efer = unsafe { read_msr(EFER_MSR) };
	efer |= EFER_NXE;
	unsafe { write_msr(EFER_MSR, efer) };
	unsafe {
		core::arch::asm!(
			"mov cr0, {}",
			in(reg) cr0 | CR0_WRITE_PROTECT,
			options(nomem, nostack, preserves_flags)
		);
	}

	let root = construir_mapa_identidade(five_level_paging)?;
	unsafe {
		core::arch::asm!(
			"mov cr3, {}",
			in(reg) root as u64,
			options(nostack, preserves_flags)
		);
	}

	let ranges = [
		(
			core::ptr::addr_of!(__text_start) as u64,
			core::ptr::addr_of!(__text_end) as u64,
			Permissions {
				writable: false,
				user: false,
				no_execute: false,
			},
		),
		(
			core::ptr::addr_of!(__rodata_start) as u64,
			core::ptr::addr_of!(__rodata_end) as u64,
			Permissions {
				writable: false,
				user: false,
				no_execute: true,
			},
		),
		(
			core::ptr::addr_of!(__data_start) as u64,
			core::ptr::addr_of!(__data_end) as u64,
			Permissions {
				writable: true,
				user: false,
				no_execute: true,
			},
		),
	];
	for (start, end, permissions) in ranges {
		proteger_intervalo(start, end, permissions)?;
	}
	configurar_cache_nao_cacheavel(boot_info.frame_buffer as u64, frame_buffer_size)?;
	unsafe {
		core::arch::asm!(
			"mov cr3, {}",
			in(reg) root as u64,
			options(nostack, preserves_flags)
		);
	}
	Ok(())
}

pub(super) fn mapear_identidade(start: u64, size: usize) -> Result<(), ErroVirtual> {
	if size == 0 {
		return Ok(());
	}
	let end = start
		.checked_add(size as u64)
		.ok_or(ErroVirtual::InvalidRange)?;
	let first_page = start & !(PAGE_SIZE - 1);
	let end_page = end
		.checked_add(PAGE_SIZE - 1)
		.ok_or(ErroVirtual::InvalidRange)?
		& !(PAGE_SIZE - 1);
	let mut address = first_page;
	while address < end_page {
		let pte = obter_pte(address, true)?;
		let entry = unsafe { ptr::read_volatile(pte) };
		if entry & PRESENT != 0 {
			if entry & PAGE_MASK != address {
				return Err(ErroVirtual::ConflictingMapping);
			}
		} else {
			unsafe {
				ptr::write_volatile(pte, address | PRESENT | WRITABLE | NO_EXECUTE);
				core::arch::asm!(
					"invlpg [{}]",
					in(reg) address,
					options(nostack, preserves_flags)
				);
			}
		}
		address = address
			.checked_add(PAGE_SIZE)
			.ok_or(ErroVirtual::InvalidRange)?;
	}
	Ok(())
}

pub(super) fn mapear_dispositivo(start: u64, size: usize) -> Result<(), ErroVirtual> {
	validar_intervalo(start, size)?;
	mapear_identidade(start, size)?;
	configurar_cache_nao_cacheavel(start, size)
}

fn proteger_intervalo(
	start: u64,
	end: u64,
	permissions: Permissions,
) -> Result<(), ErroVirtual> {
	if start >= end || start % PAGE_SIZE != 0 || end % PAGE_SIZE != 0 {
		return Err(ErroVirtual::InvalidRange);
	}
	let mut address = start;
	while address < end {
		let pte = obter_pte(address, false)?;
		let mut entry = unsafe { ptr::read_volatile(pte) };
		if entry & PRESENT == 0 {
			return Err(ErroVirtual::UnmappedPage);
		}
		entry = (entry & !(WRITABLE | USER | NO_EXECUTE))
			| if permissions.writable { WRITABLE } else { 0 }
			| if permissions.user { USER } else { 0 }
			| if permissions.no_execute {
				NO_EXECUTE
			} else {
				0
			};
		unsafe {
			ptr::write_volatile(pte, entry);
			core::arch::asm!("invlpg [{}]", in(reg) address, options(nostack, preserves_flags));
		}
		address += PAGE_SIZE;
	}
	Ok(())
}

fn validar_intervalo(start: u64, size: usize) -> Result<(), ErroVirtual> {
	let end = start
		.checked_add(size as u64)
		.ok_or(ErroVirtual::InvalidRange)?;
	if size == 0 || end > MAX_PHYSICAL_ADDRESS {
		return Err(ErroVirtual::InvalidRange);
	}
	Ok(())
}

fn construir_mapa_identidade(five_level_paging: bool) -> Result<*mut u64, ErroVirtual> {
	unsafe {
		NEXT_PAGE_TABLE = 0;
	}
	let root = novo_page_table()?;
	let pml4 = if five_level_paging {
		let pml4 = novo_page_table()?;
		unsafe {
			ptr::write_volatile(root, pml4 as u64 | PRESENT | WRITABLE);
		}
		pml4
	} else {
		root
	};
	let pdpt = novo_page_table()?;
	unsafe {
		ptr::write_volatile(pml4, pdpt as u64 | PRESENT | WRITABLE);
	}

	for pdpt_index in 0..PAGE_DIRECTORIES {
		let directory = novo_page_table()?;
		unsafe {
			ptr::write_volatile(
				pdpt.add(pdpt_index),
				directory as u64 | PRESENT | WRITABLE,
			);
		}
		for pde_index in 0..512 {
			let page_address =
				((pdpt_index * 512 + pde_index) as u64).checked_mul(TWO_MIB)
					.ok_or(ErroVirtual::InvalidRange)?;
			unsafe {
				ptr::write_volatile(
					directory.add(pde_index),
					page_address | PRESENT | WRITABLE | LARGE_PAGE | NO_EXECUTE,
				);
			}
		}
	}
	let text_start = core::ptr::addr_of!(__text_start) as u64;
	let text_end = core::ptr::addr_of!(__text_end) as u64;
	let mut address = text_start;
	while address < text_end {
		let pte = obter_pte_direto(root, address, five_level_paging)?;
		unsafe {
			ptr::write_volatile(
				pte,
				address | PRESENT | WRITABLE,
			);
		}
		address += PAGE_SIZE;
	}
	Ok(root)
}

fn configurar_cache_nao_cacheavel(start: u64, size: usize) -> Result<(), ErroVirtual> {
	let end = start
		.checked_add(size as u64)
		.ok_or(ErroVirtual::InvalidRange)?;
	let mut address = start & !(PAGE_SIZE - 1);
	let end_page = end
		.checked_add(PAGE_SIZE - 1)
		.ok_or(ErroVirtual::InvalidRange)?
		& !(PAGE_SIZE - 1);
	while address < end_page {
		let pte = obter_pte(address, false)?;
		let entry = unsafe { ptr::read_volatile(pte) };
		if entry & PRESENT == 0 || entry & PAGE_MASK != address {
			return Err(ErroVirtual::UnmappedPage);
		}
		unsafe {
			ptr::write_volatile(pte, entry | WRITE_THROUGH | CACHE_DISABLE);
			core::arch::asm!("invlpg [{}]", in(reg) address, options(nostack, preserves_flags));
		}
		address = address
			.checked_add(PAGE_SIZE)
			.ok_or(ErroVirtual::InvalidRange)?;
	}
	Ok(())
}


fn obter_pte_direto(
	root: *mut u64,
	address: u64,
	five_level_paging: bool,
) -> Result<*mut u64, ErroVirtual> {
	let root = if five_level_paging {
		descend(
			unsafe { root.add(((address >> 48) & 0x1ff) as usize) },
			false,
		)?
	} else {
		root
	};
	let pml4e = unsafe { root.add(((address >> 39) & 0x1ff) as usize) };
	let pdpt = descend(pml4e, false)?;
	let pdpte = unsafe { pdpt.add(((address >> 30) & 0x1ff) as usize) };
	if unsafe { ptr::read_volatile(pdpte) } & LARGE_PAGE != 0 {
		return Err(ErroVirtual::InvalidRange);
	}
	let directory = descend(pdpte, false)?;
	let pde = unsafe { directory.add(((address >> 21) & 0x1ff) as usize) };
	if unsafe { ptr::read_volatile(pde) } & LARGE_PAGE != 0 {
		split_2m(pde)?;
	}
	let pte = descend(pde, false)?;
	Ok(unsafe { pte.add(((address >> 12) & 0x1ff) as usize) })
}

fn obter_pte(address: u64, create: bool) -> Result<*mut u64, ErroVirtual> {
	let cr3: u64;
	unsafe {
		core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
	}
	let root = (cr3 & PAGE_MASK) as *mut u64;
	if root.is_null() || address >> 48 != 0 {
		return Err(ErroVirtual::InvalidRange);
	}
	let cr4: u64;
	unsafe {
		core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
	}
	let root = if cr4 & CR4_LA57 != 0 {
		descend(
			unsafe { root.add(((address >> 48) & 0x1ff) as usize) },
			create,
		)?
	} else {
		root
	};

	let pml4e = unsafe { root.add(((address >> 39) & 0x1ff) as usize) };
	let pdpt = descend(pml4e, create)?;
	let pdpte = unsafe { pdpt.add(((address >> 30) & 0x1ff) as usize) };
	if unsafe { ptr::read_volatile(pdpte) } & LARGE_PAGE != 0 {
		split_1g(pdpte)?;
	}
	let directory = descend(pdpte, create)?;
	let pde = unsafe { directory.add(((address >> 21) & 0x1ff) as usize) };
	if unsafe { ptr::read_volatile(pde) } & LARGE_PAGE != 0 {
		split_2m(pde)?;
	}
	let pte = descend(pde, create)?;
	Ok(unsafe { pte.add(((address >> 12) & 0x1ff) as usize) })
}

fn descend(entry: *mut u64, create: bool) -> Result<*mut u64, ErroVirtual> {
	let value = unsafe { ptr::read_volatile(entry) };
	if value & PRESENT == 0 {
		if !create {
			return Err(ErroVirtual::UnmappedPage);
		}
		let table = novo_page_table()?;
		unsafe {
			ptr::write_volatile(entry, table as u64 | PRESENT | WRITABLE);
		}
		return Ok(table);
	}
	Ok((value & PAGE_MASK) as *mut u64)
}

fn split_1g(entry: *mut u64) -> Result<(), ErroVirtual> {
	let old = unsafe { ptr::read_volatile(entry) };
	if old & LARGE_PAGE == 0 {
		return Ok(());
	}
	let table = novo_page_table()?;
	let base = old & 0x000f_ffff_c000_0000;
	let flags = old & (PRESENT | WRITABLE | USER | WRITE_THROUGH | CACHE_DISABLE | ACCESSED | DIRTY | GLOBAL | NO_EXECUTE | LARGE_PAGE_PAT);
	for index in 0..512 {
		unsafe {
			ptr::write_volatile(
				table.add(index),
				base + (index as u64 * 0x20_0000) | flags | LARGE_PAGE,
			);
		}
	}
	unsafe {
		ptr::write_volatile(entry, table as u64 | PRESENT | WRITABLE | (old & USER));
	}
	Ok(())
}

fn split_2m(entry: *mut u64) -> Result<(), ErroVirtual> {
	let old = unsafe { ptr::read_volatile(entry) };
	if old & LARGE_PAGE == 0 {
		return Ok(());
	}
	let table = novo_page_table()?;
	let base = old & 0x000f_ffff_ffe0_0000;
	let flags = old & (PRESENT | WRITABLE | USER | WRITE_THROUGH | CACHE_DISABLE | ACCESSED | DIRTY | GLOBAL | NO_EXECUTE);
	let pat = if old & LARGE_PAGE_PAT != 0 { LARGE_PAGE } else { 0 };
	for index in 0..512 {
		unsafe {
			ptr::write_volatile(
				table.add(index),
				base + (index as u64 * PAGE_SIZE) | flags | pat,
			);
		}
	}
	unsafe {
		ptr::write_volatile(entry, table as u64 | PRESENT | WRITABLE | (old & USER));
	}
	Ok(())
}

fn novo_page_table() -> Result<*mut u64, ErroVirtual> {
	let index = unsafe { NEXT_PAGE_TABLE };
	if index >= MAX_PAGE_TABLES {
		return Err(ErroVirtual::PageTablePoolExhausted);
	}
	unsafe {
		NEXT_PAGE_TABLE = index + 1;
		let table = ptr::addr_of_mut!(PAGE_TABLE_POOL.0)
			.cast::<u64>()
			.add(index * 512);
		ptr::write_bytes(table, 0, 512);
		Ok(table)
	}
}

unsafe fn read_msr(msr: u32) -> u64 {
	let low: u32;
	let high: u32;
	core::arch::asm!(
		"rdmsr",
		in("ecx") msr,
		out("eax") low,
		out("edx") high,
		options(nomem, nostack, preserves_flags)
	);
	(u64::from(high) << 32) | u64::from(low)
}

unsafe fn write_msr(msr: u32, value: u64) {
	core::arch::asm!(
		"wrmsr",
		in("ecx") msr,
		in("eax") value as u32,
		in("edx") (value >> 32) as u32,
		options(nomem, nostack, preserves_flags)
	);
}

// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.14.2026
// Funcao: Valida MTRRs de RAM e configura write-combining no framebuffer quando seguro.
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, BootInfo};

const IA32_MTRRCAP: u32 = 0x0fe;
const IA32_MTRR_DEF_TYPE: u32 = 0x2ff;
const IA32_MTRR_PHYSBASE0: u32 = 0x200;
const MTRR_ENABLE: u64 = 1 << 11;
const MTRR_FIXED_ENABLE: u64 = 1 << 10;
const MTRR_FIXED_SUPPORTED: u64 = 1 << 8;
const MTRR_WC_SUPPORTED: u64 = 1 << 10;
const MTRR_VALID: u64 = 1 << 11;
const MTRR_WRITE_COMBINING: u8 = 1;
const MTRR_WRITE_BACK: u8 = 6;
const MTRR_TYPE_MASK: u64 = 0xff;
const PAGE_SIZE: u64 = 4096;
const ONE_MIB: u64 = 1024 * 1024;
const MAX_RAM_ADDRESS: u64 = 64 * 1024 * 1024 * 1024;
const MAX_RANGES: usize = 64;
const CR0_CACHE_DISABLE: u64 = 1 << 30;
const CR0_NOT_WRITE_THROUGH: u64 = 1 << 29;
static WC_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
struct MtrrRange {
	start: u64,
	end: u64,
	memory_type: u8,
}

#[derive(Clone, Copy)]
struct MtrrBlock {
	start: u64,
	size: u64,
}

pub(super) fn configurar(boot_info: &BootInfo, pat_configured: bool) -> bool {
	let active = pat_configured && unsafe { configurar_inner(boot_info) };
	WC_ACTIVE.store(active, Ordering::Relaxed);
	active
}

pub(super) fn framebuffer_write_combining() -> bool {
	WC_ACTIVE.load(Ordering::Relaxed)
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let message = if framebuffer_write_combining() {
		"Memória: RAM WB; framebuffer WC"
	} else {
		"MTRRs: firmware preservado, framebuffer UC"
	};
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} {}",
		line_number,
		message,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

unsafe fn configurar_inner(boot_info: &BootInfo) -> bool {
	let basic_leaf = core::arch::x86_64::__cpuid(1);
	if basic_leaf.edx & (1 << 12) == 0 || basic_leaf.edx & (1 << 16) == 0 {
		return false;
	}
	if core::arch::x86_64::__cpuid(0x8000_0000).eax < 0x8000_0008 {
		return false;
	}
	let physical_bits = (core::arch::x86_64::__cpuid(0x8000_0008).eax & 0xff) as u32;
	if !(36..=52).contains(&physical_bits) {
		return false;
	}
	let physical_limit = 1u64 << physical_bits;
	let physical_mask = (physical_limit - 1) & !(PAGE_SIZE - 1);
	let capabilities = read_msr(IA32_MTRRCAP);
	let variable_count = (capabilities & 0xff) as usize;
	if variable_count == 0 || capabilities & MTRR_WC_SUPPORTED == 0 {
		return false;
	}
	let default_type = read_msr(IA32_MTRR_DEF_TYPE);
	if default_type & MTRR_ENABLE == 0
		|| default_type & MTRR_TYPE_MASK != u64::from(MTRR_WRITE_BACK)
	{
		return false;
	}
	let fixed_enabled = default_type & MTRR_FIXED_ENABLE != 0;
	if fixed_enabled && capabilities & MTRR_FIXED_SUPPORTED == 0 {
		return false;
	}
	let Some((framebuffer_start, framebuffer_end)) =
		faixa_framebuffer(boot_info, physical_limit)
	else {
		return false;
	};
	if !ram_conventional_em_wb(
		boot_info,
		variable_count,
		physical_mask,
		fixed_enabled,
		framebuffer_start,
		framebuffer_end,
	)
	{
		return false;
	}
	let mut overlaps = false;
	let mut already_write_combining = false;
	for index in 0..variable_count {
		let range = match read_variable_range(index, physical_mask) {
			Ok(Some(range)) => range,
			Ok(None) => continue,
			Err(()) => return false,
		};
		if range.start < framebuffer_end && framebuffer_start < range.end {
			overlaps = true;
			if range.memory_type != MTRR_WRITE_COMBINING
				|| range.start > framebuffer_start
				|| range.end < framebuffer_end
			{
				return false;
			}
			already_write_combining = true;
		}
	}
	if overlaps {
		return already_write_combining;
	}

	let mut blocks = [MtrrBlock { start: 0, size: 0 }; MAX_RANGES];
	let Some(block_count) = decompor_intervalo(framebuffer_start, framebuffer_end, &mut blocks)
	else {
		return false;
	};
	let mut free_indices = [0u8; MAX_RANGES];
	let mut free_count = 0;
	for index in 0..variable_count {
		if read_msr(variable_mask_msr(index)) & MTRR_VALID == 0 {
			if free_count == block_count {
				break;
			}
			free_indices[free_count] = index as u8;
			free_count += 1;
		}
	}
	if free_count != block_count {
		return false;
	}

	programar_intervalos(
		default_type,
		physical_mask,
		&blocks,
		&free_indices,
		block_count,
	);
	true
}

unsafe fn ram_conventional_em_wb(
	boot_info: &BootInfo,
	variable_count: usize,
	physical_mask: u64,
	fixed_enabled: bool,
	framebuffer_start: u64,
	framebuffer_end: u64,
) -> bool {
	if boot_info.memory_map.is_null()
		|| boot_info.memory_descriptor_size < 40
		|| boot_info.memory_map_size % boot_info.memory_descriptor_size != 0
	{
		return false;
	}
	let descriptor_count = boot_info.memory_map_size / boot_info.memory_descriptor_size;
	for index in 0..descriptor_count {
		let Some(offset) = index.checked_mul(boot_info.memory_descriptor_size) else {
			return false;
		};
		let descriptor = boot_info.memory_map.add(offset);
		if core::ptr::read_unaligned(descriptor.cast::<u32>()) != 7 {
			continue;
		}
		let start = core::ptr::read_unaligned(descriptor.add(8).cast::<u64>());
		let page_count = core::ptr::read_unaligned(descriptor.add(24).cast::<u64>());
		let Some(end) = page_count
			.checked_mul(PAGE_SIZE)
			.and_then(|size| start.checked_add(size))
		else {
			return false;
		};
		if start < framebuffer_end && framebuffer_start < end {
			return false;
		}
		if start >= MAX_RAM_ADDRESS {
			continue;
		}
		let end = end.min(MAX_RAM_ADDRESS);
		if fixed_enabled && start < ONE_MIB {
			let mut address = start & !(PAGE_SIZE - 1);
			while address < end.min(ONE_MIB) {
				if fixed_memory_type(address) != MTRR_WRITE_BACK {
					return false;
				}
				address += PAGE_SIZE;
			}
		}
		for variable_index in 0..variable_count {
			let range = match read_variable_range(variable_index, physical_mask) {
				Ok(range) => range,
				Err(()) => return false,
			};
			if let Some(range) = range {
				if range.memory_type != MTRR_WRITE_BACK
					&& start < range.end
					&& range.start < end
				{
					return false;
				}
			}
		}
	}
	true
}

unsafe fn faixa_framebuffer(boot_info: &BootInfo, physical_limit: u64) -> Option<(u64, u64)> {
	let size = boot_info
		.pixels_per_scan_line
		.checked_mul(boot_info.height)?
		.checked_mul(core::mem::size_of::<u32>())? as u64;
	let start = boot_info.frame_buffer as u64;
	let raw_end = start.checked_add(size)?;
	let aligned_start = start & !(PAGE_SIZE - 1);
	let aligned_end = raw_end.checked_add(PAGE_SIZE - 1)? & !(PAGE_SIZE - 1);
	if aligned_start < ONE_MIB || aligned_end > physical_limit || aligned_start >= aligned_end {
		return None;
	}
	Some((aligned_start, aligned_end))
}

fn decompor_intervalo(
	mut start: u64,
	end: u64,
	blocks: &mut [MtrrBlock; MAX_RANGES],
) -> Option<usize> {
	let mut count = 0;
	while start < end {
		if count == blocks.len() {
			return None;
		}
		let remaining = end - start;
		let largest_by_length = 1u64 << (63 - remaining.leading_zeros());
		let largest_by_alignment = if start == 0 {
			1u64 << 63
		} else {
			1u64 << start.trailing_zeros()
		};
		let size = largest_by_length.min(largest_by_alignment);
		if size < PAGE_SIZE {
			return None;
		}
		blocks[count] = MtrrBlock { start, size };
		count += 1;
		start = start.checked_add(size)?;
	}
	Some(count)
}

unsafe fn read_variable_range(
	index: usize,
	physical_mask: u64,
) -> Result<Option<MtrrRange>, ()> {
	let base = read_msr(variable_base_msr(index));
	let mask = read_msr(variable_mask_msr(index));
	if mask & MTRR_VALID == 0 {
		return Ok(None);
	}
	let range_mask = mask & physical_mask;
	let Some(size) = (!range_mask & physical_mask).checked_add(PAGE_SIZE) else {
		return Err(());
	};
	if !size.is_power_of_two()
		|| size < PAGE_SIZE
		|| range_mask != physical_mask & !(size - 1)
	{
		return Err(());
	}
	let start = base & physical_mask;
	if start & (size - 1) != 0 {
		return Err(());
	}
	let Some(end) = start.checked_add(size) else {
		return Err(());
	};
	let memory_type = (base & MTRR_TYPE_MASK) as u8;
	if !matches!(memory_type, 0 | 1 | 4 | 5 | 6) {
		return Err(());
	}
	Ok(Some(MtrrRange {
		start,
		end,
		memory_type,
	}))
}

unsafe fn fixed_memory_type(address: u64) -> u8 {
	let (msr, index) = if address < 0x80000 {
		(0x250, (address / 0x10000) as u32)
	} else if address < 0xa0000 {
		(0x258, ((address - 0x80000) / 0x4000) as u32)
	} else if address < 0xc0000 {
		(0x259, ((address - 0xa0000) / 0x4000) as u32)
	} else {
		let offset = address - 0xc0000;
		(
			0x268 + (offset / 0x8000) as u32,
			((offset % 0x8000) / PAGE_SIZE) as u32,
		)
	};
	((read_msr(msr) >> (index * 8)) & MTRR_TYPE_MASK) as u8
}

unsafe fn programar_intervalos(
	default_type: u64,
	physical_mask: u64,
	blocks: &[MtrrBlock; MAX_RANGES],
	indices: &[u8; MAX_RANGES],
	count: usize,
) {
	core::arch::asm!("cli", options(nomem, nostack));
	core::arch::asm!("wbinvd", options(nostack, preserves_flags));
	let cr0: u64;
	core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
	core::arch::asm!(
		"mov cr0, {}",
		in(reg) (cr0 | CR0_CACHE_DISABLE) & !CR0_NOT_WRITE_THROUGH,
		options(nomem, nostack, preserves_flags)
	);
	core::arch::asm!("wbinvd", options(nostack, preserves_flags));
	write_msr(IA32_MTRR_DEF_TYPE, default_type & !MTRR_ENABLE);
	core::arch::asm!("wbinvd", options(nostack, preserves_flags));
	for index in 0..count {
		let mtrr_index = usize::from(indices[index]);
		let block = blocks[index];
		let base = (block.start & physical_mask) | u64::from(MTRR_WRITE_COMBINING);
		let mask = (physical_mask & !(block.size - 1)) | MTRR_VALID;
		write_msr(variable_base_msr(mtrr_index), base);
		write_msr(variable_mask_msr(mtrr_index), mask);
	}
	core::arch::asm!("wbinvd", options(nostack, preserves_flags));
	write_msr(IA32_MTRR_DEF_TYPE, default_type);
	core::arch::asm!("wbinvd", options(nostack, preserves_flags));
	core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack, preserves_flags));
}

fn variable_base_msr(index: usize) -> u32 {
	IA32_MTRR_PHYSBASE0 + (index as u32 * 2)
}

fn variable_mask_msr(index: usize) -> u32 {
	IA32_MTRR_PHYSBASE0 + (index as u32 * 2) + 1
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

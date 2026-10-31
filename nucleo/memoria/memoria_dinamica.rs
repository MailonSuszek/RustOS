// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.8.2026
// Funcao: Implementa o heap global do nucleo sobre paginas fisicas.
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

use super::{paginas, virtual_memory};

const PAGE_SIZE: usize = 4096;
const ALLOCATION_MAGIC: u64 = 0x5255_5354_4f53_4845;

#[repr(C)]
struct AllocationHeader {
	base: u64,
	page_count: usize,
	magic: u64,
}

struct KernelAllocator;

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroHeap {
	JaInicializado,
	TesteAlocacao,
}

static mut INITIALIZED: bool = false;

pub(super) fn inicializar() -> Result<(), ErroHeap> {
	if unsafe { INITIALIZED } {
		return Err(ErroHeap::JaInicializado);
	}
	unsafe {
		INITIALIZED = true;
	}
	let layout = Layout::from_size_align(16, 8).map_err(|_| ErroHeap::TesteAlocacao)?;
	let allocation = unsafe { ALLOCATOR.alloc(layout) };
	if allocation.is_null() {
		return Err(ErroHeap::TesteAlocacao);
	}
	unsafe {
		ptr::write_volatile(allocation, 0xA5);
		if ptr::read_volatile(allocation) != 0xA5 {
			ALLOCATOR.dealloc(allocation, layout);
			return Err(ErroHeap::TesteAlocacao);
		}
		ALLOCATOR.dealloc(allocation, layout);
	}
	Ok(())
}

unsafe impl GlobalAlloc for KernelAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		if !INITIALIZED {
			return ptr::null_mut();
		}
		let Some(required_size) = layout
			.size()
			.checked_add(core::mem::size_of::<AllocationHeader>())
			.and_then(|size| size.checked_add(layout.align() - 1))
		else {
			return ptr::null_mut();
		};
		let Some(page_count) = required_size
			.checked_add(PAGE_SIZE - 1)
			.map(|size| size / PAGE_SIZE)
		else {
			return ptr::null_mut();
		};
		let Ok(base) = paginas::alocar(page_count) else {
			return ptr::null_mut();
		};
		if virtual_memory::mapear_identidade(base, page_count * PAGE_SIZE).is_err() {
			let _ = paginas::liberar(base, page_count);
			return ptr::null_mut();
		}

		let allocation_start = base as usize + core::mem::size_of::<AllocationHeader>();
		let aligned_start = allocation_start
			.checked_add(layout.align() - 1)
			.map(|address| address & !(layout.align() - 1));
		let Some(aligned_start) = aligned_start else {
			let _ = paginas::liberar(base, page_count);
			return ptr::null_mut();
		};
		let header = (aligned_start - core::mem::size_of::<AllocationHeader>())
			as *mut AllocationHeader;
		ptr::write(
			header,
			AllocationHeader {
				base,
				page_count,
				magic: ALLOCATION_MAGIC,
			},
		);
		aligned_start as *mut u8
	}

	unsafe fn dealloc(&self, pointer: *mut u8, _layout: Layout) {
		if pointer.is_null() {
			return;
		}
		let header = pointer
			.sub(core::mem::size_of::<AllocationHeader>())
			.cast::<AllocationHeader>();
		let allocation = ptr::read(header);
		if allocation.magic != ALLOCATION_MAGIC
			|| paginas::liberar(allocation.base, allocation.page_count).is_err()
		{
			loop {
				core::hint::spin_loop();
			}
		}
	}
}
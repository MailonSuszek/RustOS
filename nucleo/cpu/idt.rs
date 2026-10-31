// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.8.2026
// Funcao: Define e carrega a tabela de descritores de interrupcao.
use core::arch::global_asm;
use core::ptr;

const VECTOR_COUNT: usize = 256;
const INTERRUPT_GATE: u8 = 0x8e;

global_asm!(
	".global idt_fatal_handler",
	".type idt_fatal_handler,@function",
	"idt_fatal_handler:",
	"cli",
	"2:",
	"hlt",
	"jmp 2b",
);

unsafe extern "C" {
	fn idt_fatal_handler();
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
	offset_low: u16,
	selector: u16,
	ist: u8,
	attributes: u8,
	offset_middle: u16,
	offset_high: u32,
	reserved: u32,
}

impl IdtEntry {
	const EMPTY: Self = Self {
		offset_low: 0,
		selector: 0,
		ist: 0,
		attributes: 0,
		offset_middle: 0,
		offset_high: 0,
		reserved: 0,
	};

	fn interrupt_gate(handler: u64, selector: u16) -> Self {
		Self {
			offset_low: handler as u16,
			selector,
			ist: 0,
			attributes: INTERRUPT_GATE,
			offset_middle: (handler >> 16) as u16,
			offset_high: (handler >> 32) as u32,
			reserved: 0,
		}
	}
}

#[repr(C, packed(2))]
struct Idtr {
	limit: u16,
	base: u64,
}

static mut IDT: [IdtEntry; VECTOR_COUNT] = [IdtEntry::EMPTY; VECTOR_COUNT];

pub(super) fn inicializar() {
	let code_segment: u16;
	unsafe {
		core::arch::asm!(
			"mov {segment:x}, cs",
			segment = out(reg) code_segment,
			options(nomem, nostack, preserves_flags)
		);
	}

	let handler = idt_fatal_handler as *const () as usize as u64;
	let entry = IdtEntry::interrupt_gate(handler, code_segment);
	let idt = ptr::addr_of_mut!(IDT).cast::<IdtEntry>();
	for vector in 0..VECTOR_COUNT {
		unsafe { ptr::write(idt.add(vector), entry) };
	}

	let idtr = Idtr {
		limit: (core::mem::size_of::<IdtEntry>() * VECTOR_COUNT - 1) as u16,
		base: idt as u64,
	};
	unsafe {
		core::arch::asm!(
			"lidt [{}]",
			in(reg) &idtr,
			options(readonly, nostack, preserves_flags)
		);
	}
}

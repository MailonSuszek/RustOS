// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Define e carrega a tabela de descritores de interrupcao.
use core::arch::global_asm;
use core::ptr;
use core::sync::atomic::{AtomicU64, Ordering};

const VECTOR_COUNT: usize = 256;
const NMI_VECTOR: usize = 2;
const INTERRUPT_GATE: u8 = 0x8e;
pub(super) const CODE_SELECTOR: u16 = 0x08;

#[repr(C, packed(2))]
struct GdtPointer {
	limit: u16,
	base: u64,
}

static GDT: [u64; 4] = [
	0,
	0x00af_9b00_0000_ffff,
	0x00cf_9300_0000_ffff,
	0x00cf_9b00_0000_ffff,
];

global_asm!(
	".global idt_fatal_handler",
	".type idt_fatal_handler,@function",
	"idt_fatal_handler:",
	"cli",
	"2:",
	"hlt",
	"jmp 2b",
	".global idt_nmi_handler",
	".type idt_nmi_handler,@function",
	"idt_nmi_handler:",
	"lock inc qword ptr [rip + IDT_NMI_COUNT]",
	"iretq",
	".global idt_load_gdt",
	".type idt_load_gdt,@function",
	"idt_load_gdt:",
	"lgdt [rdi]",
	"push 0x08",
	"lea rax, [rip + 1f]",
	"push rax",
	".byte 0x48, 0xcb",
	"1:",
	"mov ax, 0x10",
	"mov ds, ax",
	"mov es, ax",
	"mov ss, ax",
	"ret",
);

unsafe extern "C" {
	fn idt_fatal_handler();
	fn idt_nmi_handler();
	fn idt_load_gdt(pointer: *const GdtPointer);
}

#[no_mangle]
static IDT_NMI_COUNT: AtomicU64 = AtomicU64::new(0);

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
	let gdtr = GdtPointer {
		limit: (core::mem::size_of_val(&GDT) - 1) as u16,
		base: GDT.as_ptr() as u64,
	};
	unsafe { idt_load_gdt(&gdtr) };

	let handler = idt_fatal_handler as *const () as usize as u64;
	let entry = IdtEntry::interrupt_gate(handler, CODE_SELECTOR);
	let idt = ptr::addr_of_mut!(IDT).cast::<IdtEntry>();
	for vector in 0..VECTOR_COUNT {
		unsafe { ptr::write(idt.add(vector), entry) };
	}
	let nmi = IdtEntry::interrupt_gate(
		idt_nmi_handler as *const () as usize as u64,
		CODE_SELECTOR,
	);
	unsafe { ptr::write(idt.add(NMI_VECTOR), nmi) };

	carregar();
}

pub(super) fn carregar() {
	let idt = ptr::addr_of!(IDT).cast::<IdtEntry>();
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

pub(super) fn nmis_recebidos() -> u64 {
	IDT_NMI_COUNT.load(Ordering::Relaxed)
}

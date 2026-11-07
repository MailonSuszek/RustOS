// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.14.2026
// Funcao: Define e carrega a tabela de descritores de interrupcao e a TSS.
use core::arch::global_asm;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::BootInfo;

const VECTOR_COUNT: usize = 256;
const NMI_VECTOR: usize = 2;
const INTERRUPT_GATE: u8 = 0x8e;
pub(super) const CODE_SELECTOR: u16 = 0x08;
const TSS_INDEX: usize = 4;
const MAX_CPUS: usize = 256;
// Cada processador recebe um descritor TSS proprio dentro da mesma GDT.
const GDT_ENTRIES: usize = TSS_INDEX + MAX_CPUS * 2;
const IST_NMI: u8 = 1;
const IST_FATAL: u8 = 2;

#[repr(C, packed(2))]
struct GdtPointer {
	limit: u16,
	base: u64,
}

static mut GDT: [u64; GDT_ENTRIES] = [0; GDT_ENTRIES];

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Tss {
	reserved0: u32,
	rsp0: u64,
	rsp1: u64,
	rsp2: u64,
	reserved1: u64,
	ist: [u64; 7],
	reserved2: u64,
	reserved3: u16,
	iomap_base: u16,
}

#[repr(C, align(16))]
struct IstEmergencia([u8; 8192]);

static mut TSS: [Tss; MAX_CPUS] = [Tss {
	reserved0: 0,
	rsp0: 0,
	rsp1: 0,
	rsp2: 0,
	reserved1: 0,
	ist: [0; 7],
	reserved2: 0,
	reserved3: 0,
	iomap_base: 0,
}; MAX_CPUS];
// Pilhas IST do BSP. Os APs recebem pilhas proprias alocadas antes do SIPI.
static mut IST_EMERGENCIA: IstEmergencia = IstEmergencia([0; 8192]);
static INICIALIZADO: AtomicBool = AtomicBool::new(false);
static BOOT_INFO: AtomicU64 = AtomicU64::new(0);

global_asm!(
	".global idt_fatal_handler",
	".type idt_fatal_handler,@function",
	"idt_fatal_handler:",
	"cli",
	"and rsp, -16",
	"call idt_excecao_fatal",
	"2:",
	"hlt",
	"jmp 2b",
	".macro EXCEPTION_NO_ERROR number",
	".global idt_exception_\\number",
	"idt_exception_\\number:",
	"push 0",
	"push \\number",
	"jmp idt_exception_common",
	".endm",
	".macro EXCEPTION_ERROR number",
	".global idt_exception_\\number",
	"idt_exception_\\number:",
	"push \\number",
	"jmp idt_exception_common",
	".endm",
	"EXCEPTION_NO_ERROR 0",
	"EXCEPTION_NO_ERROR 1",
	"EXCEPTION_NO_ERROR 2",
	"EXCEPTION_NO_ERROR 3",
	"EXCEPTION_NO_ERROR 4",
	"EXCEPTION_NO_ERROR 5",
	"EXCEPTION_NO_ERROR 6",
	"EXCEPTION_NO_ERROR 7",
	"EXCEPTION_ERROR 8",
	"EXCEPTION_NO_ERROR 9",
	"EXCEPTION_ERROR 10",
	"EXCEPTION_ERROR 11",
	"EXCEPTION_ERROR 12",
	"EXCEPTION_ERROR 13",
	"EXCEPTION_ERROR 14",
	"EXCEPTION_NO_ERROR 15",
	"EXCEPTION_NO_ERROR 16",
	"EXCEPTION_ERROR 17",
	"EXCEPTION_NO_ERROR 18",
	"EXCEPTION_NO_ERROR 19",
	"EXCEPTION_NO_ERROR 20",
	"EXCEPTION_ERROR 21",
	"EXCEPTION_NO_ERROR 22",
	"EXCEPTION_NO_ERROR 23",
	"EXCEPTION_NO_ERROR 24",
	"EXCEPTION_NO_ERROR 25",
	"EXCEPTION_NO_ERROR 26",
	"EXCEPTION_NO_ERROR 27",
	"EXCEPTION_NO_ERROR 28",
	"EXCEPTION_ERROR 29",
	"EXCEPTION_ERROR 30",
	"EXCEPTION_NO_ERROR 31",
	"idt_exception_common:",
	"cli",
	"mov rdi, qword ptr [rsp]",
	"mov rsi, qword ptr [rsp + 8]",
	"mov rdx, qword ptr [rsp + 16]",
	"xor ecx, ecx",
	"cmp rdi, 14",
	"jne 3f",
	"mov rcx, cr2",
	"3:",
	"and rsp, -16",
	"call idt_excecao_com_detalhes",
	"4:",
	"hlt",
	"jmp 4b",
	".section .rodata.idt_exception_handlers,\"a\",@progbits",
	".global idt_exception_handlers",
	"idt_exception_handlers:",
	".quad idt_exception_0, idt_exception_1, idt_exception_2, idt_exception_3",
	".quad idt_exception_4, idt_exception_5, idt_exception_6, idt_exception_7",
	".quad idt_exception_8, idt_exception_9, idt_exception_10, idt_exception_11",
	".quad idt_exception_12, idt_exception_13, idt_exception_14, idt_exception_15",
	".quad idt_exception_16, idt_exception_17, idt_exception_18, idt_exception_19",
	".quad idt_exception_20, idt_exception_21, idt_exception_22, idt_exception_23",
	".quad idt_exception_24, idt_exception_25, idt_exception_26, idt_exception_27",
	".quad idt_exception_28, idt_exception_29, idt_exception_30, idt_exception_31",
	".text",
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
	static idt_exception_handlers: [usize; 32];
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

	fn interrupt_gate(handler: u64, selector: u16, ist: u8) -> Self {
		Self {
			offset_low: handler as u16,
			selector,
			ist: ist & 0x07,
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
	if INICIALIZADO.swap(true, Ordering::AcqRel) {
		return;
	}
	let base = ptr::addr_of!(IST_EMERGENCIA) as u64;
	let nmi_stack = base + 4096;
	let fatal_stack = base + 8192;
	inicializar_gdt();
	preparar_tss(0, nmi_stack, fatal_stack);
	carregar_gdt();
	carregar_tss(0);

	let exception_handlers = ptr::addr_of!(idt_exception_handlers).cast::<usize>();
	let fatal_handler = idt_fatal_handler as *const () as usize as u64;
	let idt = ptr::addr_of_mut!(IDT).cast::<IdtEntry>();
	for vector in 0..VECTOR_COUNT {
		let handler = if vector < 32 {
			unsafe { ptr::read(exception_handlers.add(vector)) as u64 }
		} else {
			fatal_handler
		};
		let entry = IdtEntry::interrupt_gate(
			handler,
			CODE_SELECTOR,
			if vector == NMI_VECTOR {
				IST_NMI
			} else {
				IST_FATAL
			},
		);
		unsafe { ptr::write(idt.add(vector), entry) };
	}
	let nmi = IdtEntry::interrupt_gate(
		idt_nmi_handler as *const () as usize as u64,
		CODE_SELECTOR,
		IST_NMI,
	);
	unsafe { ptr::write(idt.add(NMI_VECTOR), nmi) };

	carregar();
}

pub(super) fn preparar_ap(slot: u32, nmi_stack_top: u64, fatal_stack_top: u64) -> bool {
	let slot = slot as usize;
	if slot == 0 || slot >= MAX_CPUS {
		return false;
	}
	if !preparar_tss(slot, nmi_stack_top, fatal_stack_top) {
		return false;
	}
	carregar_gdt();
	carregar_tss(slot);
	carregar();
	true
}

fn inicializar_gdt() {
	let gdt = ptr::addr_of_mut!(GDT).cast::<u64>();
	unsafe {
		ptr::write_volatile(gdt.add(1), 0x00af_9b00_0000_ffff);
		ptr::write_volatile(gdt.add(2), 0x00cf_9300_0000_ffff);
		ptr::write_volatile(gdt.add(3), 0x0040_9b00_0000_ffff);
	}
}

fn preparar_tss(slot: usize, nmi_stack_top: u64, fatal_stack_top: u64) -> bool {
	if slot >= MAX_CPUS
		|| nmi_stack_top == 0
		|| fatal_stack_top == 0
		|| nmi_stack_top & 0xf != 0
		|| fatal_stack_top & 0xf != 0
	{
		return false;
	}
	let mut ist = [0u64; 7];
	ist[0] = nmi_stack_top;
	ist[1] = fatal_stack_top;
	let tss = Tss {
		reserved0: 0,
		rsp0: 0,
		rsp1: 0,
		rsp2: 0,
		reserved1: 0,
		ist,
		reserved2: 0,
		reserved3: 0,
		iomap_base: core::mem::size_of::<Tss>() as u16,
	};
	let tss_pointer = unsafe { ptr::addr_of_mut!(TSS).cast::<Tss>().add(slot) };
	unsafe { ptr::write_volatile(tss_pointer, tss) };
	escrever_descriptor_tss(slot, tss_pointer as u64);
	true
}

fn escrever_descriptor_tss(slot: usize, base: u64) {
	let limit = (core::mem::size_of::<Tss>() - 1) as u64;
	let low = (limit & 0xffff)
		| ((base & 0xffff) << 16)
		| (((base >> 16) & 0xff) << 32)
		| (0x89u64 << 40)
		| (((base >> 24) & 0xff) << 56);
	let high = base >> 32;
	let gdt = ptr::addr_of_mut!(GDT).cast::<u64>();
	let descriptor = TSS_INDEX + slot * 2;
	unsafe {
		ptr::write_volatile(gdt.add(descriptor), low);
		ptr::write_volatile(gdt.add(descriptor + 1), high);
	}
}

fn seletor_tss(slot: usize) -> u16 {
	((TSS_INDEX + slot * 2) * core::mem::size_of::<u64>()) as u16
}

fn carregar_gdt() {
	let gdtr = GdtPointer {
		limit: (core::mem::size_of::<[u64; GDT_ENTRIES]>() - 1) as u16,
		base: ptr::addr_of!(GDT) as u64,
	};
	unsafe { idt_load_gdt(&gdtr) };
}

fn carregar_tss(slot: usize) {
	unsafe {
		core::arch::asm!(
			"ltr ax",
			in("ax") seletor_tss(slot),
			options(nomem, nostack, preserves_flags)
		);
	}
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

pub(super) fn registrar_boot_info(boot_info: &BootInfo) {
	BOOT_INFO.store(boot_info as *const BootInfo as u64, Ordering::Release);
}

pub(super) fn nmis_recebidos() -> u64 {
	IDT_NMI_COUNT.load(Ordering::Relaxed)
}

#[no_mangle]
extern "C" fn idt_excecao_fatal() -> ! {
	let boot_info = BOOT_INFO.load(Ordering::Acquire);
	if boot_info != 0 {
		let boot_info = unsafe { &*(boot_info as *const BootInfo) };
		super::exibir_tela_erro(
			boot_info,
			"FALHA FATAL: EXCECAO DE CPU",
			"Relatorio limpo para exibir esta falha.",
		);
	}
	loop {
		unsafe {
			core::arch::asm!("cli", "hlt", options(nomem, nostack));
		}
	}

	#[no_mangle]
	extern "C" fn idt_excecao_com_detalhes(
		vector: u64,
		error_code: u64,
		rip: u64,
		cr2: u64,
	) -> ! {
		let boot_info = BOOT_INFO.load(Ordering::Acquire);
		if boot_info != 0 {
			let boot_info = unsafe { &*(boot_info as *const BootInfo) };
			super::exibir_tela_excecao(boot_info, vector, error_code, rip, cr2);
		}
		loop {
			unsafe {
				core::arch::asm!("cli", "hlt", options(nomem, nostack));
			}
		}
	}
}

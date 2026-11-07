// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.14.2026
// Funcao: Inicia APs habilitados na MADT usando o protocolo INIT/SIPI.
use core::arch::global_asm;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::smp_context::{
	ApInicializacao, AP_FALHOU, AP_INICIALIZADO, AP_INICIANDO, AP_PENDENTE,
};
use super::{apic, idt, ioapic, paginas, tsc, virtual_memory, BootInfo};

const MAX_PROCESSADORES: usize = 256;
const MAX_APS: usize = MAX_PROCESSADORES - 1;
pub(super) const AP_STARTUP_ENABLED: bool = false;
const AP_STACK_PAGES: usize = 2;
const AP_IST_PAGES: usize = 2;
const SIPI_BASE: u32 = 0x600;
const INIT_ASSERT: u32 = 0x0000_c500;
const INIT_DEASSERT: u32 = 0x0000_8500;
const STARTUP_POLLS_PER_MICROSECOND: u64 = 400;
const AP_START_TIMEOUT_US: u64 = 50_000;
const AP_SECOND_START_TIMEOUT_US: u64 = 100_000;
const TRAMPOLINE_LIMIT: u64 = 0x000a_0000;

global_asm!(
	".section .rodata.ap_trampoline,\"a\",@progbits",
	".balign 16",
	".global ap_trampoline_start",
	"ap_trampoline_start:",
	".code16",
	"cli",
	"cld",
	"xor ax, ax",
	"mov ds, ax",
	"mov es, ax",
	"mov ss, ax",
	"mov sp, 0x0ff0",
	"mov bx, cs",
	"movzx ebx, bx",
	"shl ebx, 4",
	".byte 0x2e, 0x0f, 0x01, 0x16",
	".word AP_GDTR_OFFSET",
	"mov eax, dword ptr [ebx + AP_CR4_OFFSET]",
	"and eax, 0x1020",
	"or eax, 0x20",
	"mov cr4, eax",
	"mov eax, dword ptr [ebx + AP_CR3_OFFSET]",
	"mov cr3, eax",
	"mov ecx, 0xc0000080",
	"rdmsr",
	"or eax, 0x900",
	"wrmsr",
	"mov eax, dword ptr [ebx + AP_CR0_OFFSET]",
	"and eax, 0x9fffffff",
	"or eax, 0x80010001",
	"mov cr0, eax",
	".byte 0xea",
	".word AP_PROTECTED_OFFSET",
	".word 0x18",
	".code32",
	"ap_trampoline_protected:",
	"mov ax, 0x10",
	"mov ds, ax",
	"mov es, ax",
	"mov ss, ax",
	"mov esp, ebx",
	"add esp, 0x0ff0",
	"push 0x08",
	"lea eax, [ebx + AP_LONG_MODE_OFFSET]",
	"push eax",
	"retf",
	".code64",
	"ap_trampoline_long_mode:",
	"mov rax, qword ptr [rip + ap_trampoline_cr4]",
	"mov cr4, rax",
	"mov ax, 0x10",
	"mov ds, ax",
	"mov es, ax",
	"mov ss, ax",
	"mov rsp, qword ptr [rip + ap_trampoline_stack]",
	"mov rdi, qword ptr [rip + ap_trampoline_context]",
	"mov rax, qword ptr [rip + ap_trampoline_entry]",
	"xor rbp, rbp",
	"jmp rax",
	".balign 8",
	".global ap_trampoline_gdt",
	"ap_trampoline_gdt:",
	".quad 0",
	".quad 0x00af9a000000ffff",
	".quad 0x00cf92000000ffff",
	".quad 0x00409a000000ffff",
	".global ap_trampoline_gdtr",
	"ap_trampoline_gdtr:",
	".word 31",
	".long 0",
	".balign 8",
	".global ap_trampoline_cr3",
	"ap_trampoline_cr3:",
	".long 0",
	".global ap_trampoline_cr0",
	"ap_trampoline_cr0:",
	".long 0",
	".balign 8",
	".global ap_trampoline_cr4",
	"ap_trampoline_cr4:",
	".quad 0",
	".global ap_trampoline_stack",
	"ap_trampoline_stack:",
	".quad 0",
	".global ap_trampoline_context",
	"ap_trampoline_context:",
	".quad 0",
	".global ap_trampoline_entry",
	"ap_trampoline_entry:",
	".quad 0",
	".global ap_trampoline_end",
	"ap_trampoline_end:",
	".set AP_GDTR_OFFSET, ap_trampoline_gdtr - ap_trampoline_start",
	".set AP_CR3_OFFSET, ap_trampoline_cr3 - ap_trampoline_start",
	".set AP_CR0_OFFSET, ap_trampoline_cr0 - ap_trampoline_start",
	".set AP_CR4_OFFSET, ap_trampoline_cr4 - ap_trampoline_start",
	".set AP_PROTECTED_OFFSET, ap_trampoline_protected - ap_trampoline_start",
	".set AP_LONG_MODE_OFFSET, ap_trampoline_long_mode - ap_trampoline_start",
	".previous",
);

unsafe extern "C" {
	static ap_trampoline_start: u8;
	static ap_trampoline_gdt: u8;
	static ap_trampoline_gdtr: u8;
	static ap_trampoline_cr3: u8;
	static ap_trampoline_cr0: u8;
	static ap_trampoline_cr4: u8;
	static ap_trampoline_stack: u8;
	static ap_trampoline_context: u8;
	static ap_trampoline_entry: u8;
	static ap_trampoline_end: u8;
}

static AP_CONTEXTOS: [ApInicializacao; MAX_APS] =
	[const { ApInicializacao::novo() }; MAX_APS];
static AP_COUNT: AtomicUsize = AtomicUsize::new(0);
static AP_EXPECTED: AtomicUsize = AtomicUsize::new(0);
static AP_FAILURES: AtomicUsize = AtomicUsize::new(0);
static NMI_SOURCE_COUNT: AtomicUsize = AtomicUsize::new(0);
static NMI_PROCESSOR_COUNT: AtomicUsize = AtomicUsize::new(0);
static NMI_CONFIGURED_PROCESSORS: AtomicUsize = AtomicUsize::new(0);
static NMI_CONFIGURATION_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, PartialEq, Eq)]
enum EsperaAp {
	Inicializado,
	Falhou,
	Iniciando,
	Pendente,
}

pub(super) fn inicializar(
	boot_info: &BootInfo,
	mut mostrar_progresso: impl FnMut(usize, usize),
) {
	AP_COUNT.store(0, Ordering::Relaxed);
	AP_EXPECTED.store(0, Ordering::Relaxed);
	AP_FAILURES.store(0, Ordering::Relaxed);
	NMI_SOURCE_COUNT.store(0, Ordering::Relaxed);
	NMI_PROCESSOR_COUNT.store(
		boot_info.processadores_uefi_habilitados,
		Ordering::Relaxed,
	);
	NMI_CONFIGURED_PROCESSORS.store(0, Ordering::Relaxed);
	NMI_CONFIGURATION_FAILURES.store(
		boot_info.processadores_uefi_habilitados,
		Ordering::Relaxed,
	);
	if !AP_STARTUP_ENABLED {
		return;
	}

	let bsp_id = match apic::id_local() {
		Ok(id) => id,
		Err(_) => {
			let expected = boot_info.processadores_uefi_habilitados.saturating_sub(1);
			AP_EXPECTED.store(expected, Ordering::Relaxed);
			AP_FAILURES.store(expected, Ordering::Relaxed);
			return;
		}
	};
	let mut processors = [ioapic::ProcessadorMadt { apic_id: 0, uid: 0 }; MAX_PROCESSADORES];
	let processor_count = match ioapic::processadores(boot_info.acpi_rsdp, &mut processors) {
		Ok(count) => count,
		Err(_) => {
			let expected = boot_info.processadores_uefi_habilitados.saturating_sub(1);
			AP_EXPECTED.store(expected, Ordering::Relaxed);
			AP_FAILURES.store(expected, Ordering::Relaxed);
			return;
		}
	};
	NMI_PROCESSOR_COUNT.store(processor_count, Ordering::Relaxed);
	NMI_CONFIGURATION_FAILURES.store(processor_count, Ordering::Relaxed);
	if !processors[..processor_count]
		.iter()
		.any(|processor| processor.apic_id == bsp_id)
	{
		let expected = boot_info
			.processadores_uefi_habilitados
			.saturating_sub(1)
			.max(processor_count);
		AP_EXPECTED.store(expected, Ordering::Relaxed);
		AP_FAILURES.store(expected, Ordering::Relaxed);
		return;
	}
	let madt_expected = processors[..processor_count]
		.iter()
		.filter(|processor| processor.apic_id != bsp_id)
		.count();
	let uefi_expected = boot_info.processadores_uefi_habilitados.saturating_sub(1);
	let expected = madt_expected.max(uefi_expected);
	AP_EXPECTED.store(expected, Ordering::Relaxed);
	AP_FAILURES.store(expected - madt_expected, Ordering::Relaxed);
	if madt_expected > MAX_APS || expected > MAX_APS {
		AP_FAILURES.store(expected, Ordering::Relaxed);
		return;
	}

	let mut nmi_sources = [ioapic::NMI_SOURCE_EMPTY; ioapic::NMI_SOURCE_CAPACITY];
	let nmi_source_count = match ioapic::fontes_nmi(boot_info.acpi_rsdp, &mut nmi_sources) {
		Ok(count) => count,
		Err(_) => 0,
	};
	NMI_SOURCE_COUNT.store(nmi_source_count, Ordering::Relaxed);
	NMI_CONFIGURATION_FAILURES.store(0, Ordering::Relaxed);

	let mut context_count = 0usize;
	for processor in processors[..processor_count].iter().copied() {
		let nmi_config = configuracao_nmi(processor.uid, &nmi_sources[..nmi_source_count]);
		if processor.apic_id == bsp_id {
			aplicar_nmi(nmi_config);
			continue;
		}
		let context = &AP_CONTEXTOS[context_count];
		context.apic_id.store(processor.apic_id, Ordering::Relaxed);
		context.nmi_lint_mask.store(nmi_config.0, Ordering::Relaxed);
		context.nmi_lint0_flags.store(nmi_config.1, Ordering::Relaxed);
		context.nmi_lint1_flags.store(nmi_config.2, Ordering::Relaxed);
		context.slot.store((context_count + 1) as u32, Ordering::Relaxed);
		context.estado.store(AP_PENDENTE, Ordering::Relaxed);
		context_count += 1;
	}
	if context_count == 0 {
		return;
	}

	let trampoline = boot_info.ap_trampoline_address;
	if trampoline == 0
		|| trampoline
			.checked_add(4096)
			.map_or(true, |end| end > TRAMPOLINE_LIMIT)
		|| trampoline % 4096 != 0
	{
		AP_FAILURES.store(expected, Ordering::Relaxed);
		AP_COUNT.store(context_count, Ordering::Release);
		return;
	}
	let source = ptr::addr_of!(ap_trampoline_start);
	let source_end = ptr::addr_of!(ap_trampoline_end);
	let trampoline_size = source_end as usize - source as usize;
	if trampoline_size == 0 || trampoline_size > 4096 {
		AP_FAILURES.store(expected, Ordering::Relaxed);
		AP_COUNT.store(context_count, Ordering::Release);
		return;
	}
	unsafe { ptr::copy_nonoverlapping(source, trampoline as *mut u8, trampoline_size) };

	let cr3: u64;
	let cr4: u64;
	let cr0: u64;
	unsafe {
		core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
		core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
		core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
	}
	let Some(cr3) = u32::try_from(cr3 & !0xfff).ok() else {
		AP_FAILURES.store(expected, Ordering::Relaxed);
		AP_COUNT.store(context_count, Ordering::Release);
		return;
	};

	for (index, context) in AP_CONTEXTOS[..context_count].iter().enumerate() {
		mostrar_progresso(index + 1, context_count);
		if !iniciar_ap(trampoline, cr3, cr0 as u32, cr4, context) {
			context.estado.store(AP_FALHOU, Ordering::Release);
		}
	}
	AP_COUNT.store(context_count, Ordering::Release);
}

pub(super) fn quantidade_inicializados(_: &BootInfo) -> usize {
	AP_CONTEXTOS[..AP_COUNT.load(Ordering::Acquire)]
		.iter()
		.filter(|context| context.estado.load(Ordering::Acquire) == AP_INICIALIZADO)
		.count()
}

pub(super) fn quantidade_falhos(_: &BootInfo) -> usize {
	AP_FAILURES.load(Ordering::Acquire)
		+ AP_CONTEXTOS[..AP_COUNT.load(Ordering::Acquire)]
			.iter()
			.filter(|context| context.estado.load(Ordering::Acquire) == AP_FALHOU)
			.count()
}

pub(super) fn quantidade_esperada(_: &BootInfo) -> usize {
	AP_EXPECTED.load(Ordering::Acquire)
}

pub(super) fn fontes_nmi(_: &BootInfo) -> usize {
	NMI_SOURCE_COUNT.load(Ordering::Acquire)
}

pub(super) fn processadores_nmi_configurados(_: &BootInfo) -> usize {
	NMI_CONFIGURED_PROCESSORS.load(Ordering::Acquire)
}

pub(super) fn processadores_nmi_esperados(_: &BootInfo) -> usize {
	NMI_PROCESSOR_COUNT.load(Ordering::Acquire)
}

pub(super) fn falhas_nmi(_: &BootInfo) -> usize {
	NMI_CONFIGURATION_FAILURES.load(Ordering::Acquire)
}

fn configuracao_nmi(uid: u32, sources: &[ioapic::FonteNmi]) -> (u32, u32, u32) {
	let mut mask = 0u32;
	let mut lint0 = 0u32;
	let mut lint1 = 0u32;
	for source in sources
		.iter()
		.filter(|source| source.processor_uid == u32::MAX || source.processor_uid == uid)
	{
		let bit = 1u32 << source.lint;
		let flags = u32::from(source.flags);
		if mask & bit != 0 {
			// Firmware as vezes repete a mesma fonte com flags divergentes.
			// Mantemos a primeira regra em vez de invalidar toda a configuracao.
			continue;
		}
		mask |= bit;
		if source.lint == 0 {
			lint0 = flags;
		} else {
			lint1 = flags;
		}
	}
	(mask, lint0, lint1)
}

fn aplicar_nmi(configuration: (u32, u32, u32)) {
	match apic::configurar_nmi_lint(
		configuration.0,
		configuration.1 as u16,
		configuration.2 as u16,
	) {
		Ok(count) if count > 0 => {
			NMI_CONFIGURED_PROCESSORS.fetch_add(1, Ordering::Relaxed);
		}
		Ok(_) => {}
		Err(_) => {
			NMI_CONFIGURATION_FAILURES.fetch_add(1, Ordering::Relaxed);
		}
	}
}

fn iniciar_ap(
	trampoline: u64,
	cr3: u32,
	cr0: u32,
	cr4: u64,
	context: &ApInicializacao,
) -> bool {
	let stack = match paginas::alocar(AP_STACK_PAGES) {
		Ok(stack) => stack,
		Err(_) => return false,
	};
	let stack_size = AP_STACK_PAGES as u64 * paginas::PAGE_SIZE;
	if virtual_memory::mapear_identidade(stack, stack_size as usize).is_err() {
		return false;
	}
	let ist = match paginas::alocar(AP_IST_PAGES) {
		Ok(ist) => ist,
		Err(_) => return false,
	};
	let ist_size = AP_IST_PAGES as u64 * paginas::PAGE_SIZE;
	if virtual_memory::mapear_identidade(ist, ist_size as usize).is_err() {
		return false;
	}
	context
		.nmi_stack_top
		.store(ist + paginas::PAGE_SIZE, Ordering::Release);
	context
		.fatal_stack_top
		.store(ist + 2 * paginas::PAGE_SIZE, Ordering::Release);

	let start_offset = ptr::addr_of!(ap_trampoline_start) as usize;
	let gdt_offset = ptr::addr_of!(ap_trampoline_gdt) as usize - start_offset;
	let gdtr_offset = ptr::addr_of!(ap_trampoline_gdtr) as usize - start_offset;
	let cr3_offset = ptr::addr_of!(ap_trampoline_cr3) as usize - start_offset;
	let cr0_offset = ptr::addr_of!(ap_trampoline_cr0) as usize - start_offset;
	let cr4_offset = ptr::addr_of!(ap_trampoline_cr4) as usize - start_offset;
	let stack_offset = ptr::addr_of!(ap_trampoline_stack) as usize - start_offset;
	let context_offset = ptr::addr_of!(ap_trampoline_context) as usize - start_offset;
	let entry_offset = ptr::addr_of!(ap_trampoline_entry) as usize - start_offset;
	unsafe {
		let gdt32 = (trampoline as *mut u8).add(gdt_offset + 24).cast::<u64>();
		let mut descriptor = ptr::read_unaligned(gdt32);
		let base = trampoline as u32;
		descriptor &= !((0xffffu64 << 16) | (0xffu64 << 32) | (0xffu64 << 56));
		descriptor |= u64::from(base & 0xffff) << 16;
		descriptor |= u64::from((base >> 16) & 0xff) << 32;
		descriptor |= u64::from((base >> 24) & 0xff) << 56;
		ptr::write_unaligned(gdt32, descriptor);
		ptr::write_unaligned(
			(trampoline as *mut u8).add(gdtr_offset + 2).cast::<u32>(),
			trampoline as u32 + gdt_offset as u32,
		);
		ptr::write_unaligned((trampoline as *mut u8).add(cr3_offset).cast::<u32>(), cr3);
		ptr::write_unaligned((trampoline as *mut u8).add(cr0_offset).cast::<u32>(), cr0);
		ptr::write_unaligned((trampoline as *mut u8).add(cr4_offset).cast::<u64>(), cr4);
		ptr::write_unaligned(
			(trampoline as *mut u8).add(stack_offset).cast::<u64>(),
			stack + stack_size - 8,
		);
		ptr::write_unaligned(
			(trampoline as *mut u8).add(context_offset).cast::<u64>(),
			context as *const ApInicializacao as u64,
		);
		ptr::write_unaligned(
			(trampoline as *mut u8).add(entry_offset).cast::<u64>(),
			inicializar_ap as *const () as u64,
		);
	}

	// Garante que o codigo e os parametros gravados na memoria sejam vistos
	// pelo AP mesmo em maquinas com caches nao coerentes entre nucleos.
	unsafe {
		core::arch::asm!("mfence", "wbinvd", options(nostack));
	}

	let vector = (trampoline >> 12) as u32;
	let apic_id = context.apic_id.load(Ordering::Acquire);
	if vector > 0xff
		|| apic::enviar_ipi(apic_id, INIT_ASSERT).is_err()
		|| !esperar_microsegundos(10_000)
		|| apic::enviar_ipi(apic_id, INIT_DEASSERT).is_err()
		|| !esperar_microsegundos(10_000)
		|| apic::enviar_ipi(apic_id, SIPI_BASE | vector).is_err()
		|| !esperar_microsegundos(200)
	{
		return false;
	}

	match aguardar_ap(context, AP_START_TIMEOUT_US) {
		EsperaAp::Inicializado => true,
		EsperaAp::Falhou => false,
		EsperaAp::Iniciando => {
			matches!(
				aguardar_ap(context, AP_SECOND_START_TIMEOUT_US),
				EsperaAp::Inicializado
			)
		}
		EsperaAp::Pendente => {
			if apic::enviar_ipi(apic_id, SIPI_BASE | vector).is_err() {
				return false;
			}
			matches!(
				aguardar_ap(context, AP_SECOND_START_TIMEOUT_US),
				EsperaAp::Inicializado
			)
		}
	}
}

fn esperar_microsegundos(microsegundos: u64) -> bool {
	if tsc::esperar_microsegundos(microsegundos) {
		return true;
	}
	let iteracoes = microsegundos
		.saturating_mul(1_000)
		.min(100_000_000);
	for _ in 0..iteracoes {
		core::hint::spin_loop();
	}
	true
}

fn aguardar_ap(context: &ApInicializacao, microsegundos: u64) -> EsperaAp {
	let ciclos = tsc::frequencia_hz()
		.and_then(|frequencia| frequencia.checked_mul(microsegundos))
		.map(|total| total / 1_000_000);
	let inicio = unsafe { core::arch::x86_64::_rdtsc() };
	let mut polls = 0u64;
	let mut iniciando = false;
	let max_polls = microsegundos.saturating_mul(STARTUP_POLLS_PER_MICROSECOND);
	loop {
		match context.estado.load(Ordering::Acquire) {
			AP_INICIALIZADO => return EsperaAp::Inicializado,
			AP_FALHOU => return EsperaAp::Falhou,
			AP_INICIANDO => iniciando = true,
			_ => {}
		}
		polls = polls.saturating_add(1);
		if polls >= max_polls {
			break;
		}
		if let Some(ciclos) = ciclos {
			if unsafe { core::arch::x86_64::_rdtsc() }.wrapping_sub(inicio) >= ciclos {
				break;
			}
		}
		core::hint::spin_loop();
	}
	if iniciando {
		EsperaAp::Iniciando
	} else {
		EsperaAp::Pendente
	}
}

extern "C" fn inicializar_ap(context_pointer: *mut ApInicializacao) -> ! {
	let context = unsafe { &*context_pointer };
	context.estado.store(AP_INICIANDO, Ordering::Release);
	unsafe {
		core::arch::asm!("cli", options(nomem, nostack));
	}
	let slot = context.slot.load(Ordering::Acquire);
	let nmi_stack_top = context.nmi_stack_top.load(Ordering::Acquire);
	let fatal_stack_top = context.fatal_stack_top.load(Ordering::Acquire);
	if !idt::preparar_ap(slot, nmi_stack_top, fatal_stack_top)
		|| apic::inicializar_ap().is_err()
		|| apic::id_local().map_or(true, |id| {
			id != context.apic_id.load(Ordering::Acquire)
		})
	{
		context.estado.store(AP_FALHOU, Ordering::Release);
		aguardar_nmi();
	}
	let nmi_mask = context.nmi_lint_mask.load(Ordering::Acquire);
	let nmi_lint0_flags = context.nmi_lint0_flags.load(Ordering::Acquire) as u16;
	let nmi_lint1_flags = context.nmi_lint1_flags.load(Ordering::Acquire) as u16;
	match apic::configurar_nmi_lint(nmi_mask, nmi_lint0_flags, nmi_lint1_flags) {
		Ok(count) if count > 0 => {
			NMI_CONFIGURED_PROCESSORS.fetch_add(1, Ordering::Relaxed);
		}
		Ok(_) => {}
		Err(_) => {
			NMI_CONFIGURATION_FAILURES.fetch_add(1, Ordering::Relaxed);
		}
	}
	context.estado.store(AP_INICIALIZADO, Ordering::Release);
	aguardar_nmi()
}

fn aguardar_nmi() -> ! {
	loop {
		unsafe { core::arch::asm!("hlt", options(nomem, nostack)) };
	}
}

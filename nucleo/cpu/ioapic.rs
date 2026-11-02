// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Localiza e configura os controladores IOAPIC via ACPI.
use core::ptr;

use super::virtual_memory;

const RSDP_V1_SIZE: usize = 20;
const RSDP_V2_SIZE: usize = 36;
const ACPI_HEADER_SIZE: usize = 36;
const MADT_HEADER_SIZE: usize = 44;
const MAX_ACPI_TABLE_SIZE: usize = 1024 * 1024;
const MAX_IDENTITY_MAPPED_ADDRESS: u64 = 64 * 1024 * 1024 * 1024;
const IOAPIC_REGISTER_SELECT: usize = 0;
const IOAPIC_REGISTER_WINDOW: usize = 4;
const IOAPIC_VERSION_REGISTER: u32 = 1;
const IOAPIC_REDIRECTION_BASE: u32 = 0x10;
const IOAPIC_MASKED: u32 = 1 << 16;
const DEFAULT_INTERRUPT_VECTOR: u32 = 0x20;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroIoApic {
	AcpiAusente,
	TabelaAcpiInvalida,
	IoApicAusente,
	EnderecoInvalido,
	MapeamentoVirtual(virtual_memory::ErroVirtual),
}

pub(super) fn inicializar(rsdp: *const u8) -> Result<(), ErroIoApic> {
	let madt = localizar_madt(rsdp)?;
	let table_length = ler_u32(madt, 4) as usize;
	let mut offset = MADT_HEADER_SIZE;
	let mut ioapic_count = 0;

	while offset < table_length {
		if table_length - offset < 2 {
			return Err(ErroIoApic::TabelaAcpiInvalida);
		}
		let entry_type = ler_u8(madt, offset);
		let entry_length = usize::from(ler_u8(madt, offset + 1));
		if entry_length < 2 || entry_length > table_length - offset {
			return Err(ErroIoApic::TabelaAcpiInvalida);
		}
		if entry_type == 1 {
			if entry_length < 12 {
				return Err(ErroIoApic::TabelaAcpiInvalida);
			}
			let address = u64::from(ler_u32(madt, offset + 4));
			inicializar_controlador(address)?;
			ioapic_count += 1;
		}
		offset += entry_length;
	}

	if ioapic_count == 0 {
		return Err(ErroIoApic::IoApicAusente);
	}
	Ok(())
}

fn inicializar_controlador(address: u64) -> Result<(), ErroIoApic> {
	if address == 0 || address % 4096 != 0 {
		return Err(ErroIoApic::EnderecoInvalido);
	}
	virtual_memory::mapear_dispositivo(address, 4096)
		.map_err(ErroIoApic::MapeamentoVirtual)?;

	let version = unsafe { ler_registro(address, IOAPIC_VERSION_REGISTER) };
	let last_redirection_entry = (version >> 16) & 0xff;
	for index in 0..=last_redirection_entry {
		let low_register = IOAPIC_REDIRECTION_BASE + index * 2;
		unsafe {
			escrever_registro(address, low_register + 1, 0);
			escrever_registro(
				address,
				low_register,
				IOAPIC_MASKED | DEFAULT_INTERRUPT_VECTOR,
			);
		}
	}
	Ok(())
}

fn localizar_madt(rsdp: *const u8) -> Result<*const u8, ErroIoApic> {
	if rsdp.is_null() {
		return Err(ErroIoApic::AcpiAusente);
	}
	validar_intervalo(rsdp as u64, RSDP_V1_SIZE)?;
	if !bytes_iguais(rsdp, 0, b"RSD PTR ") || !checksum_valido(rsdp, RSDP_V1_SIZE) {
		return Err(ErroIoApic::TabelaAcpiInvalida);
	}

	let revision = ler_u8(rsdp, 15);
	if revision >= 2 {
		validar_intervalo(rsdp as u64, RSDP_V2_SIZE)?;
		let length = ler_u32(rsdp, 20) as usize;
		if !(RSDP_V2_SIZE..=MAX_ACPI_TABLE_SIZE).contains(&length)
			|| !checksum_valido(rsdp, length)
		{
			return Err(ErroIoApic::TabelaAcpiInvalida);
		}
		let xsdt = ler_u64(rsdp, 24);
		if xsdt != 0 {
			return localizar_tabela_raiz(xsdt, b"XSDT", 8);
		}
	}

	let rsdt = u64::from(ler_u32(rsdp, 16));
	if rsdt == 0 {
		return Err(ErroIoApic::TabelaAcpiInvalida);
	}
	localizar_tabela_raiz(rsdt, b"RSDT", 4)
}

fn localizar_tabela_raiz(
	address: u64,
	signature: &[u8; 4],
	entry_size: usize,
) -> Result<*const u8, ErroIoApic> {
	let root = address as *const u8;
	validar_tabela(root, signature)?;
	let length = ler_u32(root, 4) as usize;
	let entries_length = length
		.checked_sub(ACPI_HEADER_SIZE)
		.filter(|size| size % entry_size == 0)
		.ok_or(ErroIoApic::TabelaAcpiInvalida)?;
	for offset in (ACPI_HEADER_SIZE..ACPI_HEADER_SIZE + entries_length).step_by(entry_size) {
		let table_address = if entry_size == 8 {
			ler_u64(root, offset)
		} else {
			u64::from(ler_u32(root, offset))
		};
		if table_address == 0 {
			continue;
		}
		let table = table_address as *const u8;
		validar_intervalo(table_address, ACPI_HEADER_SIZE)?;
		if bytes_iguais(table, 0, b"APIC") {
			validar_tabela(table, b"APIC")?;
			if (ler_u32(table, 4) as usize) < MADT_HEADER_SIZE {
				return Err(ErroIoApic::TabelaAcpiInvalida);
			}
			return Ok(table);
		}
	}
	Err(ErroIoApic::IoApicAusente)
}

fn validar_tabela(table: *const u8, signature: &[u8; 4]) -> Result<(), ErroIoApic> {
	validar_intervalo(table as u64, ACPI_HEADER_SIZE)?;
	if !bytes_iguais(table, 0, signature) {
		return Err(ErroIoApic::TabelaAcpiInvalida);
	}
	let length = ler_u32(table, 4) as usize;
	if !(ACPI_HEADER_SIZE..=MAX_ACPI_TABLE_SIZE).contains(&length) {
		return Err(ErroIoApic::TabelaAcpiInvalida);
	}
	validar_intervalo(table as u64, length)?;
	if !checksum_valido(table, length) {
		return Err(ErroIoApic::TabelaAcpiInvalida);
	}
	Ok(())
}

fn validar_intervalo(address: u64, size: usize) -> Result<(), ErroIoApic> {
	let end = address
		.checked_add(size as u64)
		.filter(|end| *end <= MAX_IDENTITY_MAPPED_ADDRESS)
		.ok_or(ErroIoApic::EnderecoInvalido)?;
	if address == 0 || end <= address {
		return Err(ErroIoApic::EnderecoInvalido);
	}
	Ok(())
}

fn checksum_valido(address: *const u8, length: usize) -> bool {
	let mut sum = 0u8;
	for offset in 0..length {
		sum = sum.wrapping_add(ler_u8(address, offset));
	}
	sum == 0
}

fn bytes_iguais(address: *const u8, offset: usize, expected: &[u8]) -> bool {
	expected
		.iter()
		.enumerate()
		.all(|(index, byte)| ler_u8(address, offset + index) == *byte)
}

fn ler_u8(address: *const u8, offset: usize) -> u8 {
	unsafe { ptr::read(address.add(offset)) }
}

fn ler_u32(address: *const u8, offset: usize) -> u32 {
	unsafe { ptr::read_unaligned(address.add(offset).cast()) }
}

fn ler_u64(address: *const u8, offset: usize) -> u64 {
	unsafe { ptr::read_unaligned(address.add(offset).cast()) }
}

unsafe fn ler_registro(base: u64, index: u32) -> u32 {
	let registers = base as *mut u32;
	ptr::write_volatile(registers.add(IOAPIC_REGISTER_SELECT), index);
	ptr::read_volatile(registers.add(IOAPIC_REGISTER_WINDOW / core::mem::size_of::<u32>()))
}

unsafe fn escrever_registro(base: u64, index: u32, value: u32) {
	let registers = base as *mut u32;
	ptr::write_volatile(registers.add(IOAPIC_REGISTER_SELECT), index);
	ptr::write_volatile(
		registers.add(IOAPIC_REGISTER_WINDOW / core::mem::size_of::<u32>()),
		value,
	);
}

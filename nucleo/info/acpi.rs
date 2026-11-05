// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Identifica as tabelas ACPI recebidas do UEFI e lista cada uma em uma linha.
use core::fmt::{self, Write};

use super::super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::super::{draw_scrolling_text, BootInfo};

const RSDP_V1_SIZE: usize = 20;
const RSDP_V2_SIZE: usize = 36;
const ACPI_HEADER_SIZE: usize = 36;
const ACPI_SIGNATURE_SIZE: usize = 4;
const ACPI_OEM_OFFSET: usize = 10;
const ACPI_OEM_SIZE: usize = 6;
const MAX_ACPI_TABLE_SIZE: usize = 1024 * 1024;
const MAX_IDENTITY_MAPPED_ADDRESS: u64 = 64 * 1024 * 1024 * 1024;
const MAX_TABLES: usize = 32;
const ENTRY_SIZE_XSDT: usize = 8;
const ENTRY_SIZE_RSDT: usize = 4;

#[derive(Clone, Copy)]
struct Tabela {
	signature: [u8; ACPI_SIGNATURE_SIZE],
	address: u64,
	length: u32,
	fabricante: [u8; ACPI_OEM_SIZE],
}

impl Tabela {
	const fn novo() -> Self {
		Self {
			signature: [0; ACPI_SIGNATURE_SIZE],
			address: 0,
			length: 0,
			fabricante: [0; ACPI_OEM_SIZE],
		}
	}
}

struct Texto {
	bytes: [u8; 8],
	length: usize,
}

impl Texto {
	fn limpar(origem: &[u8]) -> Self {
		let mut bytes = [0u8; 8];
		let mut length = origem.len().min(bytes.len());
		for (index, byte) in origem.iter().take(length).enumerate() {
			bytes[index] = if (0x20..=0x7e).contains(byte) {
				*byte
			} else {
				b'?'
			};
		}
		while length > 0 && bytes[length - 1] == b' ' {
			length -= 1;
		}
		Self { bytes, length }
	}

	fn como_str(&self) -> &str {
		core::str::from_utf8(&self.bytes[..self.length]).unwrap_or("?")
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ErroAcpi {
	RsdpAusente,
	RsdpInvalido,
	RaizInvalida,
}

pub(crate) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line_number = line_number;
	let (count, tabelas) = match identificar(boot_info.acpi_rsdp) {
		Ok(resultado) => resultado,
		Err(erro) => {
			let mut line = LinhaBuffer::novo();
			write!(
				&mut line,
				"{{{:>width$}}} Tabelas ACPI: {}",
				line_number,
				mensagem(erro),
				width = LINE_NUMBER_WIDTH
			)?;
			draw_scrolling_text(boot_info, y, line.como_str()?);
			return line_number.checked_add(1).ok_or(fmt::Error);
		}
	};

	if count == 0 {
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} Tabelas ACPI: nenhuma encontrada",
			line_number,
			width = LINE_NUMBER_WIDTH
		)?;
		draw_scrolling_text(boot_info, y, line.como_str()?);
		return line_number.checked_add(1).ok_or(fmt::Error);
	}

	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Tabelas ACPI disponibilizadas pela UEFI: {}",
		line_number,
		count,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number = line_number.checked_add(1).ok_or(fmt::Error)?;

	for tabela in tabelas[..count].iter() {
		let signature = Texto::limpar(&tabela.signature);
		let fabricante = Texto::limpar(&tabela.fabricante);
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} Tabela ACPI: {}, endereço 0x{:016X}, {} bytes, fabricante {}",
			line_number,
			signature.como_str(),
			tabela.address,
			tabela.length,
			fabricante.como_str(),
			width = LINE_NUMBER_WIDTH
		)?;
		draw_scrolling_text(boot_info, y, line.como_str()?);
		line_number = line_number.checked_add(1).ok_or(fmt::Error)?;
	}
	Ok(line_number)
}

fn mensagem(erro: ErroAcpi) -> &'static str {
	match erro {
		ErroAcpi::RsdpAusente => "RSDP ausente",
		ErroAcpi::RsdpInvalido => "RSDP invalido",
		ErroAcpi::RaizInvalida => "tabela raiz invalida",
	}
}

fn identificar(rsdp: *const u8) -> Result<(usize, [Tabela; MAX_TABLES]), ErroAcpi> {
	if rsdp.is_null() {
		return Err(ErroAcpi::RsdpAusente);
	}
	if !intervalo_valido(rsdp as u64, RSDP_V1_SIZE)
		|| !bytes_iguais(rsdp, 0, b"RSD PTR ")
		|| !checksum_valido(rsdp, RSDP_V1_SIZE)
	{
		return Err(ErroAcpi::RsdpInvalido);
	}

	let revision = ler_u8(rsdp, 15);
	let mut xsdt = 0u64;
	if revision >= 2 {
		let length = ler_u32(rsdp, 20) as usize;
		if !(RSDP_V2_SIZE..=MAX_ACPI_TABLE_SIZE).contains(&length)
			|| !intervalo_valido(rsdp as u64, length)
			|| !checksum_valido(rsdp, length)
		{
			return Err(ErroAcpi::RsdpInvalido);
		}
		xsdt = ler_u64(rsdp, 24);
	}
	let rsdt = u64::from(ler_u32(rsdp, 16));

	// Prefere a XSDT de 64 bits e usa a RSDT como alternativa.
	let mut raiz = None;
	if xsdt != 0 {
		raiz = validar_raiz(xsdt, b"XSDT", ENTRY_SIZE_XSDT);
	}
	if raiz.is_none() && rsdt != 0 {
		raiz = validar_raiz(rsdt, b"RSDT", ENTRY_SIZE_RSDT);
	}
	let (root, entry_size, entries_length) = raiz.ok_or(ErroAcpi::RaizInvalida)?;

	let mut tabelas = [Tabela::novo(); MAX_TABLES];
	let mut count = 0usize;
	for offset in (ACPI_HEADER_SIZE..ACPI_HEADER_SIZE + entries_length).step_by(entry_size) {
		if count == MAX_TABLES {
			break;
		}
		let table_address = if entry_size == ENTRY_SIZE_XSDT {
			ler_u64(root, offset)
		} else {
			u64::from(ler_u32(root, offset))
		};
		if table_address == 0 {
			continue;
		}
		if let Some(tabela) = ler_tabela(table_address) {
			tabelas[count] = tabela;
			count += 1;
		}
	}
	Ok((count, tabelas))
}

fn validar_raiz(
	address: u64,
	signature: &[u8],
	entry_size: usize,
) -> Option<(*const u8, usize, usize)> {
	if !intervalo_valido(address, ACPI_HEADER_SIZE) {
		return None;
	}
	let root = address as *const u8;
	if !bytes_iguais(root, 0, signature) {
		return None;
	}
	let length = ler_u32(root, 4) as usize;
	let entries_length = length
		.checked_sub(ACPI_HEADER_SIZE)
		.filter(|size| size % entry_size == 0)?;
	if !intervalo_valido(address, length) || !checksum_valido(root, length) {
		return None;
	}
	Some((root, entry_size, entries_length))
}

fn ler_tabela(address: u64) -> Option<Tabela> {
	if !intervalo_valido(address, ACPI_HEADER_SIZE) {
		return None;
	}
	let table = address as *const u8;
	let length = ler_u32(table, 4);
	if !(ACPI_HEADER_SIZE as u32..=MAX_ACPI_TABLE_SIZE as u32).contains(&length)
		|| !intervalo_valido(address, length as usize)
		|| !checksum_valido(table, length as usize)
	{
		return None;
	}
	let mut signature = [0u8; ACPI_SIGNATURE_SIZE];
	for (index, byte) in signature.iter_mut().enumerate() {
		*byte = ler_u8(table, index);
	}
	let mut fabricante = [0u8; ACPI_OEM_SIZE];
	for (index, byte) in fabricante.iter_mut().enumerate() {
		*byte = ler_u8(table, ACPI_OEM_OFFSET + index);
	}
	Some(Tabela {
		signature,
		address,
		length,
		fabricante,
	})
}

fn intervalo_valido(address: u64, size: usize) -> bool {
	let Some(end) = address.checked_add(size as u64) else {
		return false;
	};
	address != 0 && end > address && end <= MAX_IDENTITY_MAPPED_ADDRESS
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
	unsafe { core::ptr::read(address.add(offset)) }
}

fn ler_u32(address: *const u8, offset: usize) -> u32 {
	unsafe { core::ptr::read_unaligned(address.add(offset).cast()) }
}

fn ler_u64(address: *const u8, offset: usize) -> u64 {
	unsafe { core::ptr::read_unaligned(address.add(offset).cast()) }
}

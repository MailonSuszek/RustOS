// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.13.2026
// Funcao: Identifica e le o temporizador PM descrito pela tabela ACPI FADT.
use core::ptr;

use super::{ioapic, virtual_memory};

// Offsets fixos do FADT (ACPI 1.0 em diante), iguais aos usados pelo ACPICA.
const FADT_PM_TIMER_BLOCK_OFFSET: usize = 76;
const FADT_PM_TIMER_LENGTH_OFFSET: usize = 91;
const FADT_FLAGS_OFFSET: usize = 112;
const FADT_X_PM_TIMER_BLOCK_OFFSET: usize = 208;
const FADT_HARDWARE_REDUCED: u32 = 1 << 20;
const FADT_TIMER_32_BIT: u32 = 1 << 8;
const GAS_SYSTEM_MEMORY: u8 = 0;
const GAS_SYSTEM_IO: u8 = 1;
const MAX_PHYSICAL_ADDRESS: u64 = 64 * 1024 * 1024 * 1024;
const PM_TIMER_FREQUENCY_HZ: u32 = 3_579_545;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroPmTimer {
	TabelaAusente,
	TabelaInvalida,
	AcpiHardwareReduzido,
	EnderecoInvalido,
	EspacoDeEnderecoNaoSuportado,
	MapeamentoVirtual,
}

#[derive(Clone, Copy)]
pub(super) enum EspacoPmTimer {
	PortaIo,
	Memoria,
}

#[derive(Clone, Copy)]
pub(super) struct PmTimer {
	pub address: u64,
	pub space: EspacoPmTimer,
	pub width: u8,
	pub counter: u32,
}

pub(super) fn identificar(rsdp: *const u8) -> Result<PmTimer, ErroPmTimer> {
	let fadt = ioapic::localizar_tabela(rsdp, b"FACP").map_err(|error| match error {
		ioapic::ErroIoApic::AcpiAusente | ioapic::ErroIoApic::TabelaAcpiAusente => {
			ErroPmTimer::TabelaAusente
		}
		_ => ErroPmTimer::TabelaInvalida,
	})?;
	let table_length = unsafe { fadt.add(4).cast::<u32>().read_unaligned() } as usize;
	if table_length < FADT_FLAGS_OFFSET + core::mem::size_of::<u32>() {
		return Err(ErroPmTimer::TabelaInvalida);
	}
	let flags = unsafe { fadt.add(FADT_FLAGS_OFFSET).cast::<u32>().read_unaligned() };
	if flags & FADT_HARDWARE_REDUCED != 0 {
		return Err(ErroPmTimer::AcpiHardwareReduzido);
	}
	let declared_width = if flags & FADT_TIMER_32_BIT != 0 {
		32
	} else {
		24
	};

	let mut register = None;
	if table_length >= FADT_X_PM_TIMER_BLOCK_OFFSET + 12 {
		let gas = unsafe { fadt.add(FADT_X_PM_TIMER_BLOCK_OFFSET) };
		let address_space = unsafe { gas.read() };
		let bit_width = unsafe { gas.add(1).read() };
		let bit_offset = unsafe { gas.add(2).read() };
		let access_size = unsafe { gas.add(3).read() };
		let address = unsafe { gas.add(4).cast::<u64>().read_unaligned() };
		if address != 0 {
			if bit_offset != 0 || access_size > 4 {
				return Err(ErroPmTimer::TabelaInvalida);
			}
			let width = match bit_width {
				0 => declared_width,
				24 | 32 => bit_width,
				_ => return Err(ErroPmTimer::TabelaInvalida),
			};
			let space = match address_space {
				GAS_SYSTEM_MEMORY => EspacoPmTimer::Memoria,
				GAS_SYSTEM_IO => EspacoPmTimer::PortaIo,
				_ => return Err(ErroPmTimer::EspacoDeEnderecoNaoSuportado),
			};
			register = Some((address, space, width));
		}
	}

	if register.is_none() {
		if table_length < FADT_PM_TIMER_LENGTH_OFFSET + 1 {
			return Err(ErroPmTimer::TabelaInvalida);
		}
		let legacy_port = unsafe {
			fadt
				.add(FADT_PM_TIMER_BLOCK_OFFSET)
				.cast::<u32>()
				.read_unaligned()
		};
		let register_length = unsafe { fadt.add(FADT_PM_TIMER_LENGTH_OFFSET).read() };
		if legacy_port == 0 || (register_length != 0 && register_length < 4) {
			return Err(ErroPmTimer::EnderecoInvalido);
		}
		register = Some((u64::from(legacy_port), EspacoPmTimer::PortaIo, declared_width));
	}

	let (address, space, width) = register.ok_or(ErroPmTimer::EnderecoInvalido)?;
	let raw_count = match space {
		EspacoPmTimer::PortaIo => {
			if address > u64::from(u16::MAX) || address + 4 > 0x1_0000 {
				return Err(ErroPmTimer::EnderecoInvalido);
			}
			unsafe {
				let value: u32;
				core::arch::asm!(
					"in eax, dx",
					in("dx") address as u16,
					out("eax") value,
					options(nomem, nostack, preserves_flags)
				);
				value
			}
		}
		EspacoPmTimer::Memoria => {
			if address % 4 != 0
				|| address
					.checked_add(4)
					.map_or(true, |end| end > MAX_PHYSICAL_ADDRESS)
			{
				return Err(ErroPmTimer::EnderecoInvalido);
			}
			virtual_memory::mapear_dispositivo(address, 4)
				.map_err(|_| ErroPmTimer::MapeamentoVirtual)?;
			unsafe { ptr::read_volatile(address as *const u32) }
		}
	};
	let counter = if width == 32 {
		raw_count
	} else {
		raw_count & 0x00ff_ffff
	};
	Ok(PmTimer {
		address,
		space,
		width,
		counter,
	})
}

pub(super) const fn mensagem(erro: ErroPmTimer) -> &'static str {
	match erro {
		ErroPmTimer::TabelaAusente => "FADT ausente",
		ErroPmTimer::TabelaInvalida => "FADT invalida",
		ErroPmTimer::AcpiHardwareReduzido => "ACPI hardware-reduced sem PM-Timer",
		ErroPmTimer::EnderecoInvalido => "endereco ou largura do PM-Timer invalido",
		ErroPmTimer::EspacoDeEnderecoNaoSuportado => {
			"espaco de enderecos do PM-Timer nao suportado"
		}
		ErroPmTimer::MapeamentoVirtual => "falha ao mapear o PM-Timer",
	}
}

pub(super) const fn frequencia_hz() -> u32 {
	PM_TIMER_FREQUENCY_HZ
}

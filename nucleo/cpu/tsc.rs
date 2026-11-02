// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Calibra o relogio interno do nucleo pelo TSC e exibe a frequencia detectada da CPU.
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU64, Ordering};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, BootInfo};

const PIT_FREQUENCY_HZ: u64 = 1_193_182;
const PIT_CHANNEL_2_DATA: u16 = 0x42;
const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL_2_MODE_2: u8 = 0b1011_0100;
const PIT_LATCH_CHANNEL_2: u8 = 0b1000_0000;
const PIT_LOAD_COUNT: u16 = 0xffff;
const PIT_TARGET_TICKS: u16 = 0x8000;
const PORT_61: u16 = 0x61;
const PORT_61_GATE_2: u8 = 0x01;
const PORT_61_SPEAKER_DATA: u8 = 0x02;
const MAX_POLLS: u32 = 20_000_000;
const MIN_FREQUENCY_HZ: u64 = 100_000_000;
const MAX_FREQUENCY_HZ: u64 = 20_000_000_000;
const NAO_DETECTADA: u64 = u64::MAX;

static FREQUENCIA_HZ: AtomicU64 = AtomicU64::new(0);

/// Frequencia do TSC em hertz, calibrada uma unica vez e mantida em cache.
pub(super) fn frequencia_hz() -> Option<u64> {
	match FREQUENCIA_HZ.load(Ordering::Relaxed) {
		NAO_DETECTADA => None,
		0 => {
			let frequencia = calibrar_com_pit()
				.or_else(frequencia_cpuid)
				.filter(|hz| (MIN_FREQUENCY_HZ..=MAX_FREQUENCY_HZ).contains(hz));
			FREQUENCIA_HZ.store(frequencia.unwrap_or(NAO_DETECTADA), Ordering::Relaxed);
			frequencia
		}
		hz => Some(hz),
	}
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line = LinhaBuffer::novo();
	match frequencia_hz() {
		Some(hz) => {
			let mhz = (hz + 500_000) / 1_000_000;
			let centesimos_ghz = (hz + 5_000_000) / 10_000_000;
			let ghz = centesimos_ghz / 100;
			let fracao = centesimos_ghz % 100;
			write!(
				&mut line,
				"{{{:>width$}}} Frequência detectada de {} MHz ({}.{:02} GHz)",
				line_number,
				mhz,
				ghz,
				fracao,
				width = LINE_NUMBER_WIDTH
			)?;
		}
		None => {
			write!(
				&mut line,
				"{{{:>width$}}} Frequência detectada: indisponivel",
				line_number,
				width = LINE_NUMBER_WIDTH
			)?;
		}
	}
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

/// Mede a frequencia do TSC comparando-a com o temporizador programavel (PIT).
fn calibrar_com_pit() -> Option<u64> {
	unsafe {
		let controle = ler_port(PORT_61);
		escrever_port(
			PORT_61,
			controle & !(PORT_61_GATE_2 | PORT_61_SPEAKER_DATA),
		);
		escrever_port(PIT_COMMAND, PIT_CHANNEL_2_MODE_2);
		escrever_port(PIT_CHANNEL_2_DATA, PIT_LOAD_COUNT as u8);
		escrever_port(PIT_CHANNEL_2_DATA, (PIT_LOAD_COUNT >> 8) as u8);
		escrever_port(PORT_61, (controle & !PORT_61_SPEAKER_DATA) | PORT_61_GATE_2);

		let inicio_ticks = ler_contador_pit();
		let inicio_tsc = core::arch::x86_64::_rdtsc();
		let mut tentativas = 0u32;
		let decorridos = loop {
			let atual = ler_contador_pit();
			let decorridos = inicio_ticks.wrapping_sub(atual);
			if decorridos >= PIT_TARGET_TICKS {
				break decorridos;
			}
			tentativas = tentativas.saturating_add(1);
			if tentativas >= MAX_POLLS {
				escrever_port(
					PORT_61,
					controle & !(PORT_61_GATE_2 | PORT_61_SPEAKER_DATA),
				);
				return None;
			}
			core::hint::spin_loop();
		};
		let fim_tsc = core::arch::x86_64::_rdtsc();
		escrever_port(
			PORT_61,
			controle & !(PORT_61_GATE_2 | PORT_61_SPEAKER_DATA),
		);

		let ciclos = fim_tsc.wrapping_sub(inicio_tsc);
		if ciclos == 0 || decorridos == 0 {
			return None;
		}
		let frequencia =
			(u128::from(ciclos) * u128::from(PIT_FREQUENCY_HZ)) / u128::from(decorridos);
		u64::try_from(frequencia).ok()
	}
}

/// Le o contador do canal 2 do PIT usando o comando de trava (latch).
unsafe fn ler_contador_pit() -> u16 {
	escrever_port(PIT_COMMAND, PIT_LATCH_CHANNEL_2);
	let baixo = ler_port(PIT_CHANNEL_2_DATA);
	let alto = ler_port(PIT_CHANNEL_2_DATA);
	u16::from_le_bytes([baixo, alto])
}

/// Frequencia informada pela CPUID, usada apenas se a calibracao pelo PIT falhar.
fn frequencia_cpuid() -> Option<u64> {
	let folha_maxima = core::arch::x86_64::__cpuid(0).eax;
	if folha_maxima >= 0x15 {
		let frequencia = core::arch::x86_64::__cpuid(0x15);
		if frequencia.eax != 0 && frequencia.ebx != 0 && frequencia.ecx != 0 {
			return u64::from(frequencia.ecx)
				.checked_mul(u64::from(frequencia.ebx))
				.map(|ciclos| ciclos / u64::from(frequencia.eax))
				.filter(|ciclos| *ciclos != 0);
		}
	}

	if folha_maxima >= 0x16 {
		let frequencia = core::arch::x86_64::__cpuid(0x16).eax;
		if frequencia != 0 {
			return u64::from(frequencia).checked_mul(1_000_000);
		}
	}
	None
}

unsafe fn ler_port(porta: u16) -> u8 {
	let valor: u8;
	core::arch::asm!(
		"in al, dx",
		in("dx") porta,
		out("al") valor,
		options(nomem, nostack, preserves_flags)
	);
	valor
}

unsafe fn escrever_port(porta: u16, valor: u8) {
	core::arch::asm!(
		"out dx, al",
		in("dx") porta,
		in("al") valor,
		options(nomem, nostack, preserves_flags)
	);
}

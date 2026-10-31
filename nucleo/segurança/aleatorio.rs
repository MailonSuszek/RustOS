// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.8.2026
// Funcao: Mantem o CRNG ChaCha20 do nucleo para produzir bytes imprevisiveis.
use core::fmt::{self, Write};
use core::ptr;
use core::sync::atomic::{compiler_fence, AtomicBool, Ordering};

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{draw_scrolling_text, BootInfo};

const KEY_WORDS: usize = 8;
const CACHE_SIZE: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroAleatorio {
	SementeAusente,
	NenhumaFonte,
	HardwareSemResposta,
	TpmSemResposta,
	ContadorEsgotado,
}

struct Crng {
	key: [u32; KEY_WORDS],
	counter: u64,
	cache: [u8; CACHE_SIZE],
	cache_position: usize,
	initialized: bool,
}

impl Crng {
	const fn vazio() -> Self {
		Self {
			key: [0; KEY_WORDS],
			counter: 0,
			cache: [0; CACHE_SIZE],
			cache_position: CACHE_SIZE,
			initialized: false,
		}
	}
}

struct CrngLock;

impl Drop for CrngLock {
	fn drop(&mut self) {
		LOCK.store(false, Ordering::Release);
	}
}

static LOCK: AtomicBool = AtomicBool::new(false);
static mut CRNG: Crng = Crng::vazio();

pub(super) fn inicializar(
	mut seed: [u8; 32],
	seed_status: u32,
) -> Result<(), ErroAleatorio> {
	if seed_status == 1 || seed_status == 3 {
		apagar(&mut seed);
		return Err(ErroAleatorio::NenhumaFonte);
	}
	if seed_status == 2 {
		apagar(&mut seed);
		return Err(ErroAleatorio::HardwareSemResposta);
	}
	if seed_status == 4 {
		apagar(&mut seed);
		return Err(ErroAleatorio::TpmSemResposta);
	}
	if seed.iter().all(|byte| *byte == 0) {
		apagar(&mut seed);
		return Err(ErroAleatorio::SementeAusente);
	}

	let _lock = adquirir_lock();
	let state = unsafe { &mut *ptr::addr_of_mut!(CRNG) };
	for (index, word) in state.key.iter_mut().enumerate() {
		let offset = index * 4;
		*word = u32::from_le_bytes([
			seed[offset],
			seed[offset + 1],
			seed[offset + 2],
			seed[offset + 3],
		]);
	}
	state.counter = 0;
	state.cache.fill(0);
	state.cache_position = CACHE_SIZE;
	state.initialized = true;
	apagar(&mut seed);
	drop(_lock);
	let mut probe = [0; CACHE_SIZE];
	let result = preencher(&mut probe);
	apagar(&mut probe);
	result
}

pub(super) fn preencher(output: &mut [u8]) -> Result<(), ErroAleatorio> {
	let _lock = adquirir_lock();
	let state = unsafe { &mut *ptr::addr_of_mut!(CRNG) };
	if !state.initialized {
		return Err(ErroAleatorio::SementeAusente);
	}

	let mut written = 0;
	while written < output.len() {
		if state.cache_position == CACHE_SIZE {
			gerar_bloco(state)?;
		}
		let available = CACHE_SIZE - state.cache_position;
		let count = available.min(output.len() - written);
		output[written..written + count]
			.copy_from_slice(&state.cache[state.cache_position..state.cache_position + count]);
		state.cache_position += count;
		written += count;
	}
	Ok(())
}

pub(super) fn exibir(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: usize,
) -> Result<usize, fmt::Error> {
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} Aleatoriedades do sistema ativado",
		line_number,
		width = LINE_NUMBER_WIDTH
	)?;
	draw_scrolling_text(boot_info, y, line.como_str()?);
	line_number.checked_add(1).ok_or(fmt::Error)
}

fn gerar_bloco(state: &mut Crng) -> Result<(), ErroAleatorio> {
	let next_counter = state
		.counter
		.checked_add(1)
		.ok_or(ErroAleatorio::ContadorEsgotado)?;
	let mut block = chacha20_block(&state.key, state.counter);
	state.counter = next_counter;
	for (index, word) in block.iter().take(KEY_WORDS).enumerate() {
		state.cache[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
	}
	for (index, word) in block.iter().skip(KEY_WORDS).enumerate() {
		state.key[index] = *word;
	}
	state.cache_position = 0;
	for word in &mut block {
		unsafe { ptr::write_volatile(word, 0) };
	}
	compiler_fence(Ordering::SeqCst);
	Ok(())
}

fn chacha20_block(key: &[u32; KEY_WORDS], counter: u64) -> [u32; 16] {
	let mut initial = [
		0x6170_7865,
		0x3320_646e,
		0x7962_2d32,
		0x6b20_6574,
		key[0],
		key[1],
		key[2],
		key[3],
		key[4],
		key[5],
		key[6],
		key[7],
		counter as u32,
		(counter >> 32) as u32,
		0,
		0,
	];
	let mut working = initial;
	for _ in 0..10 {
		quarter_round(&mut working, 0, 4, 8, 12);
		quarter_round(&mut working, 1, 5, 9, 13);
		quarter_round(&mut working, 2, 6, 10, 14);
		quarter_round(&mut working, 3, 7, 11, 15);
		quarter_round(&mut working, 0, 5, 10, 15);
		quarter_round(&mut working, 1, 6, 11, 12);
		quarter_round(&mut working, 2, 7, 8, 13);
		quarter_round(&mut working, 3, 4, 9, 14);
	}
	for index in 0..16 {
		working[index] = working[index].wrapping_add(initial[index]);
	}
	for word in &mut initial {
		unsafe { ptr::write_volatile(word, 0) };
	}
	compiler_fence(Ordering::SeqCst);
	working
}

fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
	state[a] = state[a].wrapping_add(state[b]);
	state[d] ^= state[a];
	state[d] = state[d].rotate_left(16);
	state[c] = state[c].wrapping_add(state[d]);
	state[b] ^= state[c];
	state[b] = state[b].rotate_left(12);
	state[a] = state[a].wrapping_add(state[b]);
	state[d] ^= state[a];
	state[d] = state[d].rotate_left(8);
	state[c] = state[c].wrapping_add(state[d]);
	state[b] ^= state[c];
	state[b] = state[b].rotate_left(7);
}

fn adquirir_lock() -> CrngLock {
	while LOCK
		.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
		.is_err()
	{
		core::hint::spin_loop();
	}
	CrngLock
}

fn apagar(bytes: &mut [u8]) {
	for byte in bytes {
		unsafe { ptr::write_volatile(byte, 0) };
	}
	compiler_fence(Ordering::SeqCst);
}

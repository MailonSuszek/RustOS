// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.12.2026
// Funcao: Seleciona e valida modos de video framebuffer oferecidos pelo GOP UEFI.

pub const PIXEL_RGB_RESERVED_8: u32 = 0;
pub const PIXEL_BGR_RESERVED_8: u32 = 1;
#[cfg(any(target_os = "uefi", test))]
pub const RESOLUCAO_PADRAO_LARGURA: u32 = 640;
#[cfg(any(target_os = "uefi", test))]
pub const RESOLUCAO_PADRAO_ALTURA: u32 = 480;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Modo {
	pub numero: u32,
	pub largura: u32,
	pub altura: u32,
	pub pixels_por_linha: u32,
	pub formato_pixel: u32,
}

impl Modo {
	pub fn tamanho_framebuffer(self) -> Option<usize> {
		if self.largura == 0
			|| self.altura == 0
			|| self.pixels_por_linha < self.largura
			|| !formato_pixel_valido(self.formato_pixel)
		{
			return None;
		}

		(self.pixels_por_linha as usize)
			.checked_mul(self.altura as usize)?
			.checked_mul(core::mem::size_of::<u32>())
	}

	#[cfg(any(target_os = "uefi", test))]
	fn e_padrao(self) -> bool {
		self.largura == RESOLUCAO_PADRAO_LARGURA
			&& self.altura == RESOLUCAO_PADRAO_ALTURA
	}

	#[cfg(any(target_os = "uefi", test))]
	fn area(self) -> u64 {
		u64::from(self.largura) * u64::from(self.altura)
	}
}

pub fn modo_valido(modo: Modo) -> bool {
	modo.tamanho_framebuffer().is_some()
}

pub const fn formato_pixel_valido(formato: u32) -> bool {
	matches!(
		formato,
		PIXEL_RGB_RESERVED_8 | PIXEL_BGR_RESERVED_8
	)
}

#[cfg(any(target_os = "uefi", test))]
pub fn selecionar_modo(atual: Option<Modo>, candidato: Modo) -> Option<Modo> {
	if !modo_valido(candidato) {
		return atual;
	}
	let Some(atual) = atual else {
		return Some(candidato);
	};

	if candidato.e_padrao() {
		return Some(candidato);
	}
	if atual.e_padrao() {
		return Some(atual);
	}

	if candidato.area() > atual.area()
		|| (candidato.area() == atual.area()
			&& (candidato.largura, candidato.altura) > (atual.largura, atual.altura))
	{
		Some(candidato)
	} else {
		Some(atual)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn modo(numero: u32, largura: u32, altura: u32) -> Modo {
		Modo {
			numero,
			largura,
			altura,
			pixels_por_linha: largura,
			formato_pixel: PIXEL_RGB_RESERVED_8,
		}
	}

	#[test]
	fn preserva_resolucao_padrao_se_disponivel() {
		let maior = modo(1, 1920, 1080);
		let padrao = modo(2, RESOLUCAO_PADRAO_LARGURA, RESOLUCAO_PADRAO_ALTURA);
		let escolhido = selecionar_modo(Some(maior), padrao);

		assert_eq!(escolhido, Some(padrao));
	}

	#[test]
	fn usa_maior_modo_compativel_se_padrao_indisponivel() {
		let pequeno = modo(0, 800, 600);
		let grande = modo(1, 1920, 1080);
		let escolhido = selecionar_modo(Some(pequeno), grande);

		assert_eq!(escolhido, Some(grande));
	}

	#[test]
	fn ignora_modos_com_stride_ou_formato_invalidos() {
		let valido = modo(0, 800, 600);
		let stride_invalido = Modo {
			pixels_por_linha: 640,
			..modo(1, 800, 600)
		};
		let formato_invalido = Modo {
			formato_pixel: 2,
			..modo(2, 1920, 1080)
		};

		assert_eq!(selecionar_modo(None, stride_invalido), None);
		assert_eq!(selecionar_modo(Some(valido), formato_invalido), Some(valido));
		assert_eq!(valido.tamanho_framebuffer(), Some(800 * 600 * 4));
	}
}
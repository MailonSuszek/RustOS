// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.5.2026
// Funcao: Coordena as etapas de inicializacao e exibicao do nucleo em ordem.
use super::{draw_text, BootInfo};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Etapa {
	Identificacao,
	Caminho,
	Seguranca,
	Memoria,
	Protecao,
}

struct Orquestrador {
	proxima: Option<Etapa>,
}

impl Orquestrador {
	const fn novo() -> Self {
		Self {
			proxima: Some(Etapa::Identificacao),
		}
	}

	fn executar(
		&mut self,
		etapa: Etapa,
		acao: impl FnOnce() -> Result<(), ErroOrdem>,
	) -> Result<(), ErroOrdem> {
		let proxima = match etapa {
			Etapa::Identificacao => Some(Etapa::Caminho),
			Etapa::Caminho => Some(Etapa::Seguranca),
			Etapa::Seguranca => Some(Etapa::Memoria),
			Etapa::Memoria => Some(Etapa::Protecao),
			Etapa::Protecao => None,
		};
		if self.proxima != Some(etapa) {
			return Err(ErroOrdem::EtapaForaDeOrdem);
		}

		acao()?;
		self.proxima = proxima;
		Ok(())
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ErroOrdem {
	EtapaForaDeOrdem,
	MapaMemoria(super::enderecos::ErroMapa),
	LinhaProtecao,
}

pub(super) fn executar(boot_info: &BootInfo) -> Result<(), ErroOrdem> {
	let mut orquestrador = Orquestrador::novo();

	orquestrador.executar(Etapa::Identificacao, || {
		draw_text(boot_info, 8, 8, "{1} RustOS Beta v0.1 Construção 0.5.2026");
		Ok(())
	})?;
	orquestrador.executar(Etapa::Caminho, || {
		draw_text(boot_info, 8, 18, "{2} Caminho: nucleo/Rust");
		Ok(())
	})?;
	orquestrador.executar(Etapa::Seguranca, || {
		draw_text(boot_info, 8, 28, "{3} Organização de segurança de execução");
		Ok(())
	})?;
	let mut y = 0;
	let mut line_number = 0;
	orquestrador.executar(Etapa::Memoria, || {
		(y, line_number) =
			super::enderecos::exibir(boot_info).map_err(ErroOrdem::MapaMemoria)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Protecao, || {
		super::o_que::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaProtecao)?;
		Ok(())
	})?;

	Ok(())
}
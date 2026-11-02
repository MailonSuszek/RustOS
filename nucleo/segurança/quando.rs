// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.9.2026
// Funcao: Coordena as etapas de inicializacao e exibicao do nucleo em ordem.
use core::fmt::Write;

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{apic, draw_scrolling_text, draw_text, ioapic, BootInfo};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Etapa {
	Identificacao,
	Caminho,
	Seguranca,
	Memoria,
	Protecao,
	Apic,
	Uefi,
	Ponteiros,
	Aleatorio,
	Video,
	SecureBoot,
	Smbios,
	Dmi,
	OperacaoMemoria,
	Tsc,
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
			Etapa::Protecao => Some(Etapa::Apic),
			Etapa::Apic => Some(Etapa::Uefi),
			Etapa::Uefi => Some(Etapa::Ponteiros),
			Etapa::Ponteiros => Some(Etapa::Aleatorio),
			Etapa::Aleatorio => Some(Etapa::Video),
			Etapa::Video => Some(Etapa::SecureBoot),
			Etapa::SecureBoot => Some(Etapa::Smbios),
			Etapa::Smbios => Some(Etapa::Dmi),
			Etapa::Dmi => Some(Etapa::OperacaoMemoria),
			Etapa::OperacaoMemoria => Some(Etapa::Tsc),
			Etapa::Tsc => None,
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
	LinhaUefi,
	LinhaProtecao,
	LinhaAleatorio,
	Apic(apic::ErroApic),
	IoApic(ioapic::ErroIoApic),
	LinhaApic,
	LinhaPonteiros,
	LinhaVideo,
	LinhaSecureBoot,
	LinhaSmbios,
	LinhaDmi,
	LinhaOperacaoMemoria,
	LinhaTsc,
}

pub(super) fn executar(boot_info: &BootInfo) -> Result<(), ErroOrdem> {
	let mut orquestrador = Orquestrador::novo();

	orquestrador.executar(Etapa::Identificacao, || {
		draw_text(boot_info, 8, 8, "{   1} RustOS Beta v0.1 Construção 0.9.2026");
		super::aguardar_linha();
		Ok(())
	})?;
	orquestrador.executar(Etapa::Caminho, || {
		draw_text(boot_info, 8, 18, "{   2} Caminho: nucleo/Rust");
		super::aguardar_linha();
		Ok(())
	})?;
	orquestrador.executar(Etapa::Seguranca, || {
		draw_text(boot_info, 8, 28, "{   3} Organização de segurança de execução");
		super::aguardar_linha();
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
		line_number = super::o_que::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaProtecao)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Apic, || {
		apic::inicializar().map_err(ErroOrdem::Apic)?;
		ioapic::inicializar(boot_info.acpi_rsdp).map_err(ErroOrdem::IoApic)?;
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} APCI iniciado",
			line_number,
			width = LINE_NUMBER_WIDTH
		)
		.map_err(|_| ErroOrdem::LinhaApic)?;
		draw_scrolling_text(boot_info, &mut y, line.como_str().map_err(|_| ErroOrdem::LinhaApic)?);
		line_number = line_number.checked_add(1).ok_or(ErroOrdem::LinhaApic)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Uefi, || {
		line_number = super::info::uefi::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaUefi)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Ponteiros, || {
		line_number = super::ponteiros::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaPonteiros)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Aleatorio, || {
		line_number = super::aleatorio::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaAleatorio)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Video, || {
		line_number = super::mapa_memoria::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaVideo)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::SecureBoot, || {
		line_number = super::secureboot::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaSecureBoot)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Smbios, || {
		line_number = super::info::smbios::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaSmbios)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Dmi, || {
		line_number = super::info::dmi::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaDmi)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::OperacaoMemoria, || {
		line_number = super::operacao::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaOperacaoMemoria)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Tsc, || {
		line_number = super::tsc::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaTsc)?;
		Ok(())
	})?;

	Ok(())
}
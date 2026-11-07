// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.14.2026
// Funcao: Coordena as etapas de inicializacao e exibicao do nucleo em ordem.
use core::fmt::Write;

use super::enderecos::{LinhaBuffer, LINE_NUMBER_WIDTH};
use super::{apic, draw_scrolling_text, draw_text, ioapic, paginas, pm_timer, BootInfo};

const GB: u64 = 1_000_000_000;

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
	ConfirmacaoPaginaZero,
	ReservaVideoBios,
	Mtrr,
	RamDisponivel,
	Pat,
	RamFisicaUtilizavel,
	PaginasGrandes,
	Ramdisk,
	TabelasAcpi,
	Bloco,
	Zonas,
	ZonaMovel,
	PaginasInacessiveis,
	SmpStatus,
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
			Etapa::Tsc => Some(Etapa::ConfirmacaoPaginaZero),
			Etapa::ConfirmacaoPaginaZero => Some(Etapa::ReservaVideoBios),
			Etapa::ReservaVideoBios => Some(Etapa::RamDisponivel),
			Etapa::RamDisponivel => Some(Etapa::Mtrr),
			Etapa::Mtrr => Some(Etapa::Pat),
			Etapa::Pat => Some(Etapa::RamFisicaUtilizavel),
			Etapa::RamFisicaUtilizavel => Some(Etapa::PaginasGrandes),
			Etapa::PaginasGrandes => Some(Etapa::Ramdisk),
			Etapa::Ramdisk => Some(Etapa::TabelasAcpi),
			Etapa::TabelasAcpi => Some(Etapa::Bloco),
			Etapa::Bloco => Some(Etapa::Zonas),
			Etapa::Zonas => Some(Etapa::ZonaMovel),
			Etapa::ZonaMovel => Some(Etapa::PaginasInacessiveis),
			Etapa::PaginasInacessiveis => Some(Etapa::SmpStatus),
			Etapa::SmpStatus => None,
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
#[allow(dead_code)]
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
	LinhaConfirmacaoPaginaZero,
	LinhaMtrr,
	LinhaRamDisponivel,
	LinhaPat,
	LinhaRamFisicaUtilizavel,
	LinhaPaginasGrandes,
	LinhaRamdisk,
	LinhaTabelasAcpi,
	Bloco(super::bloco::ErroBloco),
	LinhaBloco,
	LinhaZonas,
	ZonaMovel(super::zona_movel::ErroZonaMovel),
	LinhaZonaMovel,
	LinhaPaginasInacessiveis(super::mapa_memoria::ErroExibicaoInacessiveis),
	LinhaSmp,
}

pub(super) fn executar(boot_info: &BootInfo) -> Result<(), ErroOrdem> {
	let mut orquestrador = Orquestrador::novo();

	orquestrador.executar(Etapa::Identificacao, || {
		draw_text(boot_info, 8, 8, "{   1} RustOS Beta v0.1 Construção 0.14.2026");
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
	let mut y = 38;
	let mut line_number = 4;
	orquestrador.executar(Etapa::Memoria, || {
		line_number = super::enderecos::exibir(boot_info, &mut y, line_number)
			.map_err(ErroOrdem::MapaMemoria)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Protecao, || {
		line_number = super::o_que::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaProtecao)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Apic, || {
		let _ = apic::inicializar();
		let _ = ioapic::inicializar(boot_info.acpi_rsdp);
		let mut line = LinhaBuffer::novo();
		let _ = write!(&mut line, "{{{:>width$}}} APIC local iniciado e configurado", line_number, width = LINE_NUMBER_WIDTH);
		draw_scrolling_text(boot_info, &mut y, line.como_str().unwrap_or(""));
		line_number = line_number.saturating_add(1);
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
	orquestrador.executar(Etapa::ConfirmacaoPaginaZero, || {
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} 0000000000000000 - 0000000000000FFF (4 KB) Endereço reservado",
			line_number,
			width = LINE_NUMBER_WIDTH
		)
		.map_err(|_| ErroOrdem::LinhaConfirmacaoPaginaZero)?;
		draw_scrolling_text(
			boot_info,
			&mut y,
			line.como_str().map_err(|_| ErroOrdem::LinhaConfirmacaoPaginaZero)?,
		);
		line_number = line_number
			.checked_add(1)
			.ok_or(ErroOrdem::LinhaConfirmacaoPaginaZero)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::ReservaVideoBios, || {
		line_number = super::mapa_memoria::exibir_reserva_video_bios(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaVideo)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::RamDisponivel, || {
		let available_hundredths_gb =
			(paginas::memoria_disponivel_bytes() * 100 + GB / 2) / GB;
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} RAM disponível: {}.{:02} GB",
			line_number,
			available_hundredths_gb / 100,
			available_hundredths_gb % 100,
			width = LINE_NUMBER_WIDTH
		)
		.map_err(|_| ErroOrdem::LinhaRamDisponivel)?;
		draw_scrolling_text(
			boot_info,
			&mut y,
			line.como_str().map_err(|_| ErroOrdem::LinhaRamDisponivel)?,
		);
		line_number = line_number
			.checked_add(1)
			.ok_or(ErroOrdem::LinhaRamDisponivel)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Mtrr, || {
		line_number = super::mtrr::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaMtrr)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Pat, || {
		line_number = super::pat::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaPat)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::RamFisicaUtilizavel, || {
		let mut line = LinhaBuffer::novo();
		match paginas::memoria_fisica_utilizavel_bytes(
			boot_info.memory_map,
			boot_info.memory_map_size,
			boot_info.memory_descriptor_size,
		) {
			Some(bytes) => {
				let hundredths_gb = (bytes * 100 + GB / 2) / GB;
				write!(
					&mut line,
					"{{{:>width$}}} RAM fisica utilizavel: {}.{:02} GB",
					line_number,
					hundredths_gb / 100,
					hundredths_gb % 100,
					width = LINE_NUMBER_WIDTH
				)
				.map_err(|_| ErroOrdem::LinhaRamFisicaUtilizavel)?;
			}
			None => write!(
				&mut line,
				"{{{:>width$}}} RAM fisica utilizavel: indisponivel",
				line_number,
				width = LINE_NUMBER_WIDTH
			)
			.map_err(|_| ErroOrdem::LinhaRamFisicaUtilizavel)?,
		}
		draw_scrolling_text(
			boot_info,
			&mut y,
			line.como_str().map_err(|_| ErroOrdem::LinhaRamFisicaUtilizavel)?,
		);
		line_number = line_number
			.checked_add(1)
			.ok_or(ErroOrdem::LinhaRamFisicaUtilizavel)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::PaginasGrandes, || {
		let page_size = super::virtual_memory::tamanho_pagina_identidade();
		let message = if page_size == 1_073_741_824 {
			"Paginas grandes ativas: 1 GiB (PDPE1GB)"
		} else {
			"Paginas grandes ativas: 2 MiB (fallback)"
		};
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} {}",
			line_number,
			message,
			width = LINE_NUMBER_WIDTH
		)
		.map_err(|_| ErroOrdem::LinhaPaginasGrandes)?;
		draw_scrolling_text(
			boot_info,
			&mut y,
			line.como_str().map_err(|_| ErroOrdem::LinhaPaginasGrandes)?,
		);
		line_number = line_number
			.checked_add(1)
			.ok_or(ErroOrdem::LinhaPaginasGrandes)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Ramdisk, || {
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"{{{:>width$}}} ramdisk não encontrado",
			line_number,
			width = LINE_NUMBER_WIDTH
		)
		.map_err(|_| ErroOrdem::LinhaRamdisk)?;
		draw_scrolling_text(
			boot_info,
			&mut y,
			line.como_str().map_err(|_| ErroOrdem::LinhaRamdisk)?,
		);
		line_number = line_number.checked_add(1).ok_or(ErroOrdem::LinhaRamdisk)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::TabelasAcpi, || {
		line_number = super::info::acpi::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaTabelasAcpi)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Bloco, || {
		super::bloco::inicializar(boot_info).map_err(ErroOrdem::Bloco)?;
		line_number = super::bloco::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaBloco)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::Zonas, || {
		line_number = super::zonas::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaZonas)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::ZonaMovel, || {
		super::zona_movel::inicializar(boot_info).map_err(ErroOrdem::ZonaMovel)?;
		line_number = super::zona_movel::exibir(boot_info, &mut y, line_number)
			.map_err(|_| ErroOrdem::LinhaZonaMovel)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::PaginasInacessiveis, || {
		line_number = super::mapa_memoria::exibir_paginas_inacessiveis(
			boot_info,
			&mut y,
			line_number,
		)
		.map_err(ErroOrdem::LinhaPaginasInacessiveis)?;
		Ok(())
	})?;
	orquestrador.executar(Etapa::SmpStatus, || {
		let status = if super::smp::AP_STARTUP_ENABLED {
			"iniciando APs"
		} else {
			"APs desativados para evitar excecao"
		};
		exibir_linha_smp(boot_info, &mut y, &mut line_number, "SMP: ", status)?;
		let mut display_error = None;
		super::smp::inicializar(
			boot_info,
			|current, total| {
				if display_error.is_some() {
					return;
				}
				let mut progress = LinhaBuffer::novo();
				if write!(&mut progress, "iniciando AP {}/{}", current, total).is_err() {
					display_error = Some(ErroOrdem::LinhaSmp);
					return;
				}
				if let Err(error) =
					exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &progress)
				{
					display_error = Some(error);
				}
			},
		);
		if let Some(error) = display_error {
			return Err(error);
		}
		let initialized = super::smp::quantidade_inicializados(boot_info);
		let failed = super::smp::quantidade_falhos(boot_info);
		let expected = super::smp::quantidade_esperada(boot_info);
		let mp_services = if boot_info.ap_servico_disponivel != 0 {
			"disponivel"
		} else {
			"indisponivel"
		};
		exibir_linha_smp(boot_info, &mut y, &mut line_number, "MP Services UEFI: ", mp_services)?;
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"CPUs UEFI: {} habilitadas de {}",
			boot_info.processadores_uefi_habilitados,
			boot_info.processadores_uefi_total
		)
		.map_err(|_| ErroOrdem::LinhaSmp)?;
		exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"Trampolim de inicializacao AP: {}",
			if boot_info.ap_trampoline_address != 0 {
				"disponivel"
			} else {
				"indisponivel"
			}
		)
		.map_err(|_| ErroOrdem::LinhaSmp)?;
		exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"APs inicializados (em espera/HLT): {} de {}; falhas: {}",
			initialized,
			expected,
			failed
		)
		.map_err(|_| ErroOrdem::LinhaSmp)?;
		exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"Fontes NMI MADT: {}",
			super::smp::fontes_nmi(boot_info)
		)
		.map_err(|_| ErroOrdem::LinhaSmp)?;
		exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"Processadores com NMI: {} de {}; falhas: {}",
			super::smp::processadores_nmi_configurados(boot_info),
			super::smp::processadores_nmi_esperados(boot_info),
			super::smp::falhas_nmi(boot_info)
		)
		.map_err(|_| ErroOrdem::LinhaSmp)?;
		exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		match super::pm_timer::identificar(boot_info.acpi_rsdp) {
			Ok(timer) => {
				let mut line = LinhaBuffer::novo();
				write!(
					&mut line,
					"PM-Timer: {} 0x{:X}, {} bits, {} Hz",
					match timer.space {
						super::pm_timer::EspacoPmTimer::PortaIo => "porta I/O",
						super::pm_timer::EspacoPmTimer::Memoria => "memoria fisica",
					},
					timer.address,
					timer.width,
					super::pm_timer::frequencia_hz()
				)
				.map_err(|_| ErroOrdem::LinhaSmp)?;
				exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
				let mut line = LinhaBuffer::novo();
				write!(&mut line, "PM-Timer contador inicial: 0x{:08X}", timer.counter)
					.map_err(|_| ErroOrdem::LinhaSmp)?;
				exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
			}
			Err(error) => {
				let mut line = LinhaBuffer::novo();
				write!(
					&mut line,
					"PM-Timer: indisponivel ({})",
					super::pm_timer::mensagem(error)
				)
				.map_err(|_| ErroOrdem::LinhaSmp)?;
				exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
			}
		}
		let mut line = LinhaBuffer::novo();
		write!(
			&mut line,
			"NMIs tratados: {}",
			super::idt::nmis_recebidos()
		)
		.map_err(|_| ErroOrdem::LinhaSmp)?;
		exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		if let Ok(info) = ioapic::inicializar(boot_info.acpi_rsdp) {
			let mut line = LinhaBuffer::novo();
			let _ = write!(
				&mut line,
				"IOAPIC: ID APIC {}, versão 0x{:02X}, endereço 0x{:08X}, GSI {}-{}",
				info.id, info.version, info.address, info.gsi_base, info.gsi_end
			);
			if info.count > 1 {
				let _ = write!(&mut line, ", controladores {}", info.count);
			}
			exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		}

		if let Ok(madt) = ioapic::resumo_madt(boot_info.acpi_rsdp) {
			let mut line = LinhaBuffer::novo();
			let _ = write!(&mut line, "MADT: endereço do Local APIC 0x{:08X}", madt.local_apic_address);
			exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;

			let mut line = LinhaBuffer::novo();
			let _ = write!(&mut line, "MADT: processadores identificados e habilitados: {}", madt.processadores);
			exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;

			let mut line = LinhaBuffer::novo();
			let _ = write!(&mut line, "MADT: controladores de interrupção IOAPIC: {}", madt.ioapics);
			exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;

			let mut line = LinhaBuffer::novo();
			let _ = write!(&mut line, "MADT: redirecionamentos de interrupção (ISO): {}", madt.overrides);
			exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;

			let mut line = LinhaBuffer::novo();
			let _ = write!(&mut line, "MADT: fontes NMI configuradas: {}", madt.fontes_nmi);
			exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
		}

		match pm_timer::identificar(boot_info.acpi_rsdp) {
			Ok(timer) => {
				let espaco = match timer.space {
					pm_timer::EspacoPmTimer::PortaIo => "porta E/S",
					pm_timer::EspacoPmTimer::Memoria => "memória",
				};
				let mut line = LinhaBuffer::novo();
				let _ = write!(&mut line, "Temporizador PM: {} 0x{:X}, {} bits, {} Hz", espaco, timer.address, timer.width, pm_timer::frequencia_hz());
				exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
			}
			Err(_) => {
				let mut line = LinhaBuffer::novo();
				let _ = write!(&mut line, "Temporizador PM: não disponível");
				exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;
			}
		}

		let hpet = if ioapic::hpet_disponivel(boot_info.acpi_rsdp) { "disponível" } else { "não disponível" };
		let mut line = LinhaBuffer::novo();
		let _ = write!(&mut line, "HPET: {}", hpet);
		exibir_linha_smp_buffer(boot_info, &mut y, &mut line_number, &line)?;

		Ok(())
	})?;
	Ok(())
}

fn exibir_linha_smp(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: &mut usize,
	label: &str,
	value: &str,
) -> Result<(), ErroOrdem> {
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{}{}",
		label,
		value
	)
	.map_err(|_| ErroOrdem::LinhaSmp)?;
	exibir_linha_smp_buffer(boot_info, y, line_number, &line)
}

fn exibir_linha_smp_buffer(
	boot_info: &BootInfo,
	y: &mut usize,
	line_number: &mut usize,
	message: &LinhaBuffer,
) -> Result<(), ErroOrdem> {
	let mut line = LinhaBuffer::novo();
	write!(
		&mut line,
		"{{{:>width$}}} {}",
		*line_number,
		message.como_str().map_err(|_| ErroOrdem::LinhaSmp)?,
		width = LINE_NUMBER_WIDTH
	)
	.map_err(|_| ErroOrdem::LinhaSmp)?;
	draw_scrolling_text(
		boot_info,
		y,
		line.como_str().map_err(|_| ErroOrdem::LinhaSmp)?,
	);
	*line_number = line_number.checked_add(1).ok_or(ErroOrdem::LinhaSmp)?;
	Ok(())
}
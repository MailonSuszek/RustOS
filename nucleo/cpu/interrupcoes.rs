// Nome: RustOS Beta
// Versao: v0.1
// Construcao: 0.6.2026
// Funcao: Prepara a IDT mantendo as interrupcoes do processador desabilitadas.
pub(super) fn inicializar() {
	unsafe {
		core::arch::asm!("cli", options(nomem, nostack));
	}
	super::idt::inicializar();
}

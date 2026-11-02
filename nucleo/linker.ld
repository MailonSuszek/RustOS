/*
 * Nome: RustOS Beta
 * Versao: v0.1
 * Construcao: 0.9.2026
 * Funcao: Define o layout de memoria e os segmentos do executavel ELF do nucleo.
 */
OUTPUT_FORMAT(elf64-x86-64)
ENTRY(kernel_main)

PHDRS
{
	text PT_LOAD FLAGS(5);
	rodata PT_LOAD FLAGS(4);
	data PT_LOAD FLAGS(6);
}

SECTIONS
{
	. = 0x100000;
	__kernel_start = .;
	.text : ALIGN(4096)
	{
		__text_start = .;
		*(.text.kernel_main)
		*(.text .text.*)
		. = ALIGN(4096);
		__text_end = .;
	} :text

	.rodata : ALIGN(4096)
	{
		__rodata_start = .;
		*(.rodata .rodata.*)
		*(.data.rel.ro .data.rel.ro.*)
		. = ALIGN(4096);
		__rodata_end = .;
	} :rodata

	.data : ALIGN(4096)
	{
		__data_start = .;
		*(.data .data.*)
		*(.got .got.*)
	} :data

	.bss (NOLOAD) : ALIGN(4096)
	{
		*(.bss .bss.*)
		*(COMMON)
		. = ALIGN(4096);
		__data_end = .;
	} :data

	__kernel_end = .;

	/DISCARD/ :
	{
		*(.eh_frame .eh_frame.*)
		*(.comment)
		*(.note .note.*)
	}
}

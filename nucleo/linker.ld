OUTPUT_FORMAT(elf64-x86-64)
ENTRY(kernel_main)

PHDRS
{
	text PT_LOAD FLAGS(5);
	data PT_LOAD FLAGS(6);
}

SECTIONS
{
	. = 0x100000;
	.text : ALIGN(4096)
	{
		*(.text.kernel_main)
		*(.text .text.*)
	} :text

	.rodata : ALIGN(4096)
	{
		*(.rodata .rodata.*)
	} :text

	.data : ALIGN(4096)
	{
		*(.data .data.*)
		*(.data.rel.ro .data.rel.ro.*)
		*(.got .got.*)
	} :data

	.bss (NOLOAD) : ALIGN(4096)
	{
		*(.bss .bss.*)
		*(COMMON)
	} :data

	/DISCARD/ :
	{
		*(.eh_frame .eh_frame.*)
		*(.comment)
		*(.note .note.*)
	}
}

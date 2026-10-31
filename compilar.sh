#!/usr/bin/env bash
# Nome: RustOS Beta
# Versao: v0.1
# Construcao: 0.8.2026
# Funcao: Compila os estagios UEFI e o nucleo, depois gera e valida a ISO inicializavel.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE="${ROOT_DIR}/inicio/inimin.rs"
KERNEL_SOURCE="${ROOT_DIR}/inicio/inicio.rs"
EFI_BINARY="${ROOT_DIR}/inicio/EFI/BOOT/BOOTX64.EFI"
KERNEL_BINARY="${ROOT_DIR}/inicio/inicio.so"
RUST_KERNEL_SOURCE="${ROOT_DIR}/nucleo/nucleo.rs"
RUST_KERNEL_LINKER="${ROOT_DIR}/nucleo/linker.ld"
RUST_KERNEL_BINARY="${ROOT_DIR}/nucleo/Rust"
EFI_IMAGE="${ROOT_DIR}/inicio/inicio.img"
ISO="${ROOT_DIR}/RustOS_Beta_v0.1_Construcao_0.8.2026.iso"
PATH="/usr/local/sbin:/usr/sbin:/sbin:${PATH}"
export PATH

if [[ -f "${HOME}/.cargo/env" ]]; then
	source "${HOME}/.cargo/env"
fi

for tool in rustc rustup genisoimage isoinfo mkfs.vfat mmd mcopy; do
	if ! command -v "${tool}" >/dev/null 2>&1; then
		printf 'Erro: ferramenta obrigatoria ausente: %s\n' "${tool}" >&2
		exit 1
	fi
done

printf 'Limpando geracoes anteriores...\n'
rm -f \
	"${ROOT_DIR}"/RustOS_Beta_v0.1_Construcao_*.iso \
	"${EFI_IMAGE}" \
	"${EFI_BINARY}" \
	"${KERNEL_BINARY}" \
	"${RUST_KERNEL_BINARY}" \
	"${ROOT_DIR}/inicio/EFI/BOOT/BOOTX64.pdb" \
	"${ROOT_DIR}/inicio/inimin.efi" \
	"${ROOT_DIR}/inicio/inimin.pdb" \
	"${ROOT_DIR}/inicio/efi.img" \
	"${ROOT_DIR}/inicio.img"

for target in x86_64-unknown-uefi x86_64-unknown-none; do
	if ! rustup target list --installed | grep -qx "${target}"; then
		rustup target add "${target}"
	fi
done

mkdir -p "$(dirname "${EFI_BINARY}")"
rustc --edition=2021 --target x86_64-unknown-uefi \
	-C panic=abort -C opt-level=s -C debuginfo=0 \
	-o "${EFI_BINARY}" "${SOURCE}"
rustc --edition=2021 --target x86_64-unknown-uefi \
	-C panic=abort -C opt-level=s -C debuginfo=0 \
	-o "${KERNEL_BINARY}" "${KERNEL_SOURCE}"
rustc --edition=2021 --target x86_64-unknown-none \
	-C panic=abort -C opt-level=s -C debuginfo=0 \
	-C no-redzone=yes -C relocation-model=static -C code-model=kernel \
	-C link-arg="-T${RUST_KERNEL_LINKER}" \
	-o "${RUST_KERNEL_BINARY}" "${RUST_KERNEL_SOURCE}"

truncate -s 4M "${EFI_IMAGE}"
mkfs.vfat -F 12 -n RUSTOS "${EFI_IMAGE}" >/dev/null
mmd -i "${EFI_IMAGE}" ::/EFI
mmd -i "${EFI_IMAGE}" ::/EFI/BOOT
mcopy -i "${EFI_IMAGE}" "${EFI_BINARY}" ::/EFI/BOOT/BOOTX64.EFI
mcopy -i "${EFI_IMAGE}" "${KERNEL_BINARY}" ::/inicio.so

genisoimage -quiet -o "${ISO}" -V RUSTOS \
	-c boot.catalog \
	-e inicio/inicio.img -no-emul-boot \
	-graft-points \
	inicio/inicio.img="${EFI_IMAGE}" \
	nucleo/Rust="${RUST_KERNEL_BINARY}"

ISO_FILES="$(isoinfo -i "${ISO}" -f)"
for iso_path in "/BOOT.CAT;1" "/INICIO/INICIO.IMG;1" "/NUCLEO/RUST.;1"; do
	if ! grep -Fqx "${iso_path}" <<<"${ISO_FILES}"; then
		printf 'Erro: arquivo esperado ausente na ISO: %s\n' "${iso_path}" >&2
		exit 1
	fi
done

printf 'ISO criada: %s\n' "${ISO}"

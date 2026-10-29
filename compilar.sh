#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE="${ROOT_DIR}/inicio/inimin.rs"
EFI_BINARY="${ROOT_DIR}/inicio/EFI/BOOT/BOOTX64.EFI"
EFI_IMAGE="${ROOT_DIR}/inicio/inicio.img"
ISO="${ROOT_DIR}/RustOS_Beta_v0.1_Construcao_0.1.2026.iso"
PATH="/usr/local/sbin:/usr/sbin:/sbin:${PATH}"
export PATH

if [[ -f "${HOME}/.cargo/env" ]]; then
	source "${HOME}/.cargo/env"
fi

for tool in rustc rustup genisoimage mkfs.vfat mmd mcopy; do
	if ! command -v "${tool}" >/dev/null 2>&1; then
		printf 'Erro: ferramenta obrigatoria ausente: %s\n' "${tool}" >&2
		exit 1
	fi
done

printf 'Limpando geracoes anteriores...\n'
rm -f \
	"${ISO}" \
	"${EFI_IMAGE}" \
	"${EFI_BINARY}" \
	"${ROOT_DIR}/inicio/EFI/BOOT/BOOTX64.pdb" \
	"${ROOT_DIR}/inicio/inimin.efi" \
	"${ROOT_DIR}/inicio/inimin.pdb" \
	"${ROOT_DIR}/inicio/efi.img" \
	"${ROOT_DIR}/inicio.img"

if ! rustup target list --installed | grep -qx 'x86_64-unknown-uefi'; then
	rustup target add x86_64-unknown-uefi
fi

mkdir -p "$(dirname "${EFI_BINARY}")"
rustc --edition=2021 --target x86_64-unknown-uefi \
	-C panic=abort -C opt-level=s -C debuginfo=0 \
	-o "${EFI_BINARY}" "${SOURCE}"

truncate -s 4M "${EFI_IMAGE}"
mkfs.vfat -F 12 -n RUSTOS "${EFI_IMAGE}" >/dev/null
mmd -i "${EFI_IMAGE}" ::/EFI
mmd -i "${EFI_IMAGE}" ::/EFI/BOOT
mcopy -i "${EFI_IMAGE}" "${EFI_BINARY}" ::/EFI/BOOT/BOOTX64.EFI

genisoimage -quiet -o "${ISO}" -V RUSTOS \
	-c boot.catalog \
	-e inicio/inicio.img -no-emul-boot \
	-graft-points inicio/inicio.img="${EFI_IMAGE}"

printf 'ISO criada: %s\n' "${ISO}"

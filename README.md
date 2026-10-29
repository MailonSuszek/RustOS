# RustOS Beta

**Nome:** RustOS Beta  
**Versao:** v0.1  
**Construcao:** 0.1.2026  
**Arquitetura:** x86-64  
**Boot:** UEFI

RustOS e um projeto experimental dedicado a explorar o desenvolvimento de um sistema operacional usando Rust. Seu objetivo e evoluir gradualmente, desde a inicializacao da maquina ate os componentes fundamentais de um sistema operacional, com foco em aprendizado, confiabilidade e controle de baixo nivel.

## Estrutura

- `inicio/inimin.rs`: codigo Rust do primeiro aplicativo de boot.
- `inicio/EFI/BOOT/BOOTX64.EFI`: executavel UEFI compilado para x86-64.
- `inicio/inicio.img`: imagem FAT de boot que contem `EFI/BOOT/BOOTX64.EFI`.
- `compilar.sh`: compila o aplicativo e gera a imagem FAT e a ISO.
- `.vscode/tasks.json`: tarefas para compilar e executar no QEMU.
- `RustOS_Beta_v0.1_Construcao_0.1.2026.iso`: imagem ISO gerada na raiz do projeto.

Dentro da ISO, a imagem de boot fica em `INICIO/INICIO.IMG`; o `genisoimage` tambem cria o catalogo El Torito `BOOT.CAT` para o firmware UEFI.

## Requisitos

Em Debian ou Ubuntu, instale as ferramentas do sistema:

```bash
sudo apt update
sudo apt install -y curl qemu-system-x86 genisoimage dosfstools mtools ovmf
```

Instale o Rust com o target UEFI x86-64:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --target x86_64-unknown-uefi
```

O script adiciona o target `x86_64-unknown-uefi` automaticamente se ele ainda nao estiver instalado.

## Compilar

Na raiz do projeto, execute:

```bash
bash compilar.sh
```

O script remove as geracoes anteriores e recria o executavel UEFI, `inicio/inicio.img` e a ISO na raiz.

## Executar no QEMU

No VS Code, pressione **Ctrl+Shift+B**. A tarefa padrao compila o projeto, prepara os arquivos de firmware OVMF na raiz e inicia o QEMU pela ISO.

Os arquivos `OVMF_CODE_4M.fd` e `OVMF_VARS_4M.fd` sao firmware/configuracao do QEMU; nao fazem parte da ISO.

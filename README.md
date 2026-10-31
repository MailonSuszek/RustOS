# RustOS Beta

**Arquivo:** README.md<br>
**Funcao:** Documenta a estrutura, os requisitos, a compilacao e a execucao do RustOS.<br>
**Nome:** RustOS Beta  
**Versao:** v0.1  
**Construcao:** 0.5.2026<br>
**Arquitetura:** x86-64  
**Boot:** UEFI

RustOS e um projeto experimental dedicado a explorar o desenvolvimento de um sistema operacional usando Rust. Seu objetivo e evoluir gradualmente, desde a inicializacao da maquina ate os componentes fundamentais de um sistema operacional, com foco em aprendizado, confiabilidade e controle de baixo nivel.

## Estrutura

- `inicio/inimin.rs`: primeiro estagio UEFI; mostra “Bem vindo ao RustOS Beta”, espera 2 segundos e transfere o controle para `inicio.so` usando `LoadImage` e `StartImage`.
- `inicio/inicio.rs`: segundo estagio UEFI; configura o framebuffer, encontra `nucleo/Rust` na ISO 9660 via Block I/O, carrega seus segmentos ELF e encerra os servicos UEFI antes de transferir o controle ao nucleo.
- `inicio/EFI/BOOT/BOOTX64.EFI`: executavel UEFI compilado para x86-64.
- `inicio/inicio.so`: segundo estagio compilado para UEFI. Apesar da extensao `.so`, e um executavel PE/COFF UEFI, nao uma biblioteca ELF Linux.
- `nucleo/nucleo.rs`: codigo-fonte do nucleo freestanding x86-64.
- `nucleo/segurança/quando.rs`: orquestra em ordem as etapas de exibicao do nucleo para evitar execucao fora de sequencia.
- `nucleo/segurança/o_que.rs`: inicializa a memoria pertencente ao nucleo e exibe o estado de protecao apos a inicializacao bem-sucedida.
- `nucleo/memoria/enderecos.rs`: interpreta o mapa UEFI e lista os intervalos de RAM.
- `nucleo/memoria/paginas.rs`: aloca e libera paginas fisicas entre as regioes UEFI convencionais, com limite de 64 GiB.
- `nucleo/memoria/regioes.rs`: reserva explicitamente a imagem do nucleo, o framebuffer e os dados de boot.
- `nucleo/memoria/virtual.rs`: cria um mapeamento identidade de 64 GiB, protege as secoes do nucleo, habilita NX e CR0.WP e marca o framebuffer como nao cacheavel.
- `nucleo/memoria/heap.rs`: fornece o alocador global do nucleo baseado em paginas fisicas.
- `nucleo/Rust`: executavel ELF independente, produzido a partir de `nucleo/nucleo.rs` e incluido na pasta `NUCLEO` da ISO 9660.
- `nucleo/linker.ld`: posiciona o nucleo ELF na memoria fisica a partir de 1 MiB.
- `inicio/inicio.img`: imagem FAT de boot que contem `EFI/BOOT/BOOTX64.EFI` e `inicio.so` na raiz.
- `compilar.sh`: compila os estagios UEFI e o nucleo freestanding, e gera a imagem FAT e a ISO.
- `.vscode/tasks.json`: tarefas para compilar e executar no QEMU.
- `RustOS_Beta_v0.1_Construcao_0.5.2026.iso`: imagem ISO gerada na raiz do projeto.

Os estagios sao binarios separados: o firmware inicia `BOOTX64.EFI`, que carrega `inicio.so` de `inicio.img`; `inicio.so` procura e carrega `nucleo/Rust` diretamente dos setores da ISO 9660. O nucleo recebe do carregador os dados do framebuffer e, apos a transferencia, executa sem os servicos de boot UEFI.

A ISO contem:

```text
BOOT.CAT
INICIO/INICIO.IMG
NUCLEO/RUST
```

O nome `NUCLEO/RUST` pode aparecer como `NUCLEO/RUST.;1` ao listar a ISO, devido ao formato ISO 9660.

Na tela, o nucleo mostra:

```text
{1} RustOS Beta v0.1 Construção 0.5.2026
{2} Caminho: nucleo/Rust
{3} Organização de segurança de execução
{4} Lista de endereços da memoria RAM
{próximo número} RAM proteção ativo
```

Após os intervalos, o nucleo exibe “RAM proteção ativo” com o próximo número de linha. O núcleo inicializa um alocador de páginas físicas usando apenas memória convencional UEFI, reserva as áreas do núcleo/framebuffer/dados de boot e instala um mapeamento identidade dos primeiros 64 GiB. Código do núcleo fica somente leitura/executável; dados ficam leitura/escrita/não executáveis; o restante do mapeamento fica não executável. NX e proteção de escrita (CR0.WP) são habilitados, e o framebuffer é mapeado como não cacheável. O heap global usa páginas físicas e executa um teste de alocação na inicialização. O alocador gerencia endereços físicos abaixo de 64 GiB. Ainda não há processos de usuário nem separação de espaços de endereçamento por processo. Arquivos permanecem no armazenamento e seus dados são carregados na RAM quando usados. A listagem mostra endereços e tipos UEFI; as linhas rolam a tela quando necessário.

## Requisitos

Em Debian ou Ubuntu, instale as ferramentas do sistema:

```bash
sudo apt update
sudo apt install -y curl qemu-system-x86 genisoimage dosfstools mtools ovmf
```

Instale o Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
```

O script instala automaticamente, quando necessario, os targets `x86_64-unknown-uefi` (estagios UEFI) e `x86_64-unknown-none` (nucleo freestanding).

## Compilar

Na raiz do projeto, execute:

```bash
bash compilar.sh
```

O script remove os artefatos gerados anteriormente e recria os executaveis UEFI, o nucleo ELF em `nucleo/Rust`, `inicio/inicio.img` e a ISO na raiz. Ao final, verifica se a ISO contem o catalogo de boot, a imagem FAT e o nucleo ELF.

## Executar no QEMU

No VS Code, pressione **Ctrl+Shift+B**. A tarefa padrao compila o projeto, prepara os arquivos de firmware OVMF na raiz e inicia o QEMU pela ISO.

Os arquivos `OVMF_CODE_4M.fd` e `OVMF_VARS_4M.fd` sao firmware/configuracao do QEMU; nao fazem parte da ISO.

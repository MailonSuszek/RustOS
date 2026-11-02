# RustOS Beta

**Arquivo:** README.md<br>
**Funcao:** Documenta a estrutura, os requisitos, a compilacao e a execucao do RustOS.<br>
**Nome:** RustOS Beta  
**Versao:** v0.1  
**Construcao:** 0.9.2026<br>
**Arquitetura:** x86-64  
**Boot:** UEFI

RustOS e um projeto experimental dedicado a explorar o desenvolvimento de um sistema operacional usando Rust. Seu objetivo e evoluir gradualmente, desde a inicializacao da maquina ate os componentes fundamentais de um sistema operacional, com foco em aprendizado, confiabilidade e controle de baixo nivel.

## Estrutura

- `inicio/inimin.rs`: primeiro estagio UEFI; mostra “Bem vindo ao RustOS Beta”, espera 2 segundos e transfere o controle para `inicio.so` usando `LoadImage` e `StartImage`.
- `inicio/inicio.rs`: segundo estagio UEFI; configura o framebuffer, encontra `nucleo/Rust` na ISO 9660 via Block I/O, carrega seus segmentos ELF e encerra os servicos UEFI antes de transferir o controle ao nucleo.
- `inicio/EFI/BOOT/BOOTX64.EFI`: executavel UEFI compilado para x86-64.
- `inicio/inicio.so`: segundo estagio compilado para UEFI. Apesar da extensao `.so`, e um executavel PE/COFF UEFI, nao uma biblioteca ELF Linux.
- `nucleo/nucleo.rs`: codigo-fonte do nucleo freestanding x86-64.
- `nucleo/cpu/apic.rs`: habilita o APIC local em modo xAPIC ou x2APIC.
- `nucleo/cpu/idt.rs`: cria e carrega uma IDT com 256 vetores; os vetores ainda usam o handler fatal.
- `nucleo/cpu/ioapic.rs`: localiza os IOAPICs pela tabela ACPI MADT e mantem suas entradas mascaradas.
- `nucleo/cpu/interrupcoes.rs`: carrega a IDT sem mensagem e mantem as interrupcoes da CPU desabilitadas.
- `nucleo/cpu/tsc.rs`: calibra o TSC contra o temporizador PIT, mantem a frequencia em cache para o relogio interno e exibe a velocidade detectada da CPU.
- `nucleo/segurança/quando.rs`: orquestra em ordem as etapas de exibicao do nucleo para evitar execucao fora de sequencia.
- `nucleo/segurança/o_que.rs`: inicializa a memoria pertencente ao nucleo e exibe o estado de protecao apos a inicializacao bem-sucedida.
- `nucleo/segurança/aleatorio.rs`: inicializa um CRNG ChaCha20 com entropia do protocolo RNG UEFI, das instrucoes RDSEED/RDRAND ou do TPM 2.0 via TCG2.
- `nucleo/segurança/secureboot.rs`: exibe na ultima linha se o Secure Boot estava ativo ou desativado quando o carregador UEFI entregou o controle ao nucleo.
- `nucleo/memoria/enderecos.rs`: interpreta o mapa UEFI e lista os intervalos de RAM.
- `nucleo/memoria/paginas.rs`: aloca, reserva e libera paginas fisicas entre as regioes UEFI convencionais, com limite de 64 GiB.
- `nucleo/memoria/ponteiros.rs`: exibe os enderecos de tabelas UEFI recebidos pelo carregador, incluindo ACPI e SMBIOS.
- `nucleo/memoria/regioes.rs`: reserva explicitamente a imagem do nucleo, o framebuffer e os dados de boot.
- `nucleo/memoria/mapa_memoria.rs`: tabela unica das areas de hardware (video, ROM da BIOS, chipset, IOAPIC, HPET, Local APIC e Flash da BIOS); reserva essas faixas no alocador fisico, remove o mapeamento delas e gera as linhas correspondentes na tela.
- `nucleo/memoria/virtual.rs`: cria um mapeamento identidade de 64 GiB, protege as secoes do nucleo, habilita NX e CR0.WP, marca o framebuffer como nao cacheavel e desmapeia as areas de hardware reservadas.
- `nucleo/memoria/memoria_dinamica.rs`: fornece o alocador global do nucleo baseado em paginas fisicas.
- `nucleo/memoria/operacao.rs`: confirma pelos dados SMBIOS se todos os slots e canais de memoria estao ocupados.
- `nucleo/Rust`: executavel ELF independente, produzido a partir de `nucleo/nucleo.rs` e incluido na pasta `NUCLEO` da ISO 9660.
- `nucleo/info/uefi.rs`: exibe a versao do UEFI obtida de `SystemTable.Header.Revision` no carregador.
- `nucleo/info/smbios.rs`: exibe a versao do SMBIOS capturada pelo carregador a partir da entrada `_SM3_` ou `_SM_` da tabela de configuracao UEFI.
- `nucleo/info/dmi.rs`: exibe fabricante, modelo e BIOS do computador obtidos das estruturas DMI/SMBIOS capturadas pelo carregador.
- `nucleo/linker.ld`: posiciona o nucleo ELF na memoria fisica a partir de 1 MiB.
- `inicio/inicio.img`: imagem FAT de boot que contem `EFI/BOOT/BOOTX64.EFI` e `inicio.so` na raiz.
- `compilar.sh`: compila os estagios UEFI e o nucleo freestanding, e gera a imagem FAT e a ISO.
- `.vscode/tasks.json`: tarefas para compilar e executar no QEMU.
- `RustOS_Beta_v0.1_Construcao_0.9.2026.iso`: imagem ISO gerada na raiz do projeto.

Os estagios sao binarios separados: o firmware inicia `BOOTX64.EFI`, que carrega `inicio.so` de `inicio.img`; `inicio.so` procura e carrega `nucleo/Rust` diretamente dos setores da ISO 9660. O nucleo recebe do carregador os dados do framebuffer e, apos a transferencia, executa sem os servicos de boot UEFI.

A ISO contem:

```text
BOOT.CAT
INICIO/INICIO.IMG
NUCLEO/RUST
```

O nome `NUCLEO/RUST` pode aparecer como `NUCLEO/RUST.;1` ao listar a ISO, devido ao formato ISO 9660.

Na tela, o nucleo mostra as etapas nesta ordem:

```text
{   1} RustOS Beta v0.1 Construção 0.9.2026
{   2} Caminho: nucleo/Rust
{   3} Organização de segurança de execução
{   4} Lista de endereços da memoria RAM
{   5} <intervalo de memoria>
{próximo número} RAM proteção ativo
{próximo número} APCI iniciado
{próximo número} EFI: UEFI *numeror da versão*
{próximo número} UEFI tabelas: ACPI2=0x..., SMBIOS=0x...
{próximo número} UEFI tabelas: UEFI=0x...
{próximo número} Aleatoriedades do sistema ativado
{próximo número} 128 KB reservado para memoria de video
{próximo número} 00000000000A0000 - 00000000000BFFFF totalmente dedicado a memoria de video
{próximo número} 00000000FEC00000 - 00000000FEC10FFF (68 KB) Reserva de controle do Chipset
{próximo número} 00000000FEC00000 - 00000000FEC00FFF (4 KB) Reserva de registradores do IOAPIC
{próximo número} 00000000FED00000 - 00000000FED00FFF (4 KB) Reserva de temporizador HPET
{próximo número} 00000000FEE00000 - 00000000FEE00FFF (4 KB) Reserva de registradores do Local APIC
{próximo número} 00000000FF000000 - 00000000FFFFFFFF (16 MB) Remover a memoria Flash da BIOS
{próximo número} 00000000000C0000 - 00000000000FFFFF (256 KB) Proteção da ROM da BIOS
{próximo número} Secureboot ativo
{próximo número} SMBIOS V3.0
{próximo número} Fabricante: <fabricante>
{próximo número} Modelo: <modelo>
{próximo número} BIOS: <versao da BIOS>
{próximo número} Barramento de memoria OK: 1/1 slots e 1/1 canais ocupados
{próximo número} Frequência detectada de 2496 MHz (2.50 GHz)
```

O número da versão EFI e obtido de `SystemTable.Header.Revision`; o exemplo `2.70` pode variar conforme o firmware. Depois dessa linha, o carregador passa ao nucleo os enderecos das tabelas UEFI, identificando ACPI e SMBIOS quando reconhecidos. Todas as linhas recebem numeracao sequencial, uma por uma, inclusive as de continuacao quando a lista de tabelas nao cabe em uma unica linha. O nucleo tambem usa a MADT ACPI para localizar os IOAPICs. O CRNG usa ChaCha20 com uma semente de 256 bits obtida do protocolo RNG do UEFI, de RDSEED/RDRAND ou de TPM 2.0 via protocolo TCG2; sem uma fonte disponivel, a inicializacao falha em vez de usar uma semente previsivel.

Na ultima linha da tela, `nucleo/cpu/tsc.rs` calibra o TSC contra o PIT por cerca de 27 ms, guarda a frequencia em cache para o relogio interno e exibe a velocidade detectada. Se o PIT nao responder, a frequencia e obtida pelas folhas CPUID `0x15` e `0x16`.

Apos os intervalos, o nucleo exibe “RAM proteção ativo”, inicializa o APIC local e os IOAPICs, mostra a versao EFI, lista os ponteiros das tabelas UEFI e ativa o CRNG. A IDT esta carregada, mas todos os vetores ainda usam um handler fatal; as interrupcoes da CPU permanecem desabilitadas e as entradas do IOAPIC ficam mascaradas. Ainda nao ha tratamento ou roteamento de IRQs.

Depois da mensagem de aleatoriedade, o nucleo lista as areas de hardware reservadas, no formato `endereco inicial - endereco final (tamanho) rotulo`:

| Area | Intervalo | Tamanho |
| --- | --- | --- |
| Memoria de video (VGA) | `0x000A0000 - 0x000BFFFF` | 128 KB |
| Controle do Chipset | `0xFEC00000 - 0xFEC10FFF` | 68 KB |
| Registradores do IOAPIC | `0xFEC00000 - 0xFEC00FFF` | 4 KB |
| Temporizador HPET | `0xFED00000 - 0xFED00FFF` | 4 KB |
| Registradores do Local APIC | `0xFEE00000 - 0xFEE00FFF` | 4 KB |
| Flash da BIOS | `0xFF000000 - 0xFFFFFFFF` | 16 MB |
| ROM da BIOS | `0x000C0000 - 0x000FFFFF` | 256 KB |

Os tamanhos sao calculados a partir da propria faixa (`fim - inicio + 1`), nao digitados. A tabela `REGIOES` em `nucleo/memoria/mapa_memoria.rs` e a unica fonte para reserva fisica, desmapeamento e texto na tela, evitando divergencia entre o que e exibido e o que e feito. O IOAPIC e uma sub-regiao do bloco de controle do chipset, como no mapa real de `/proc/iomem` do Linux.

A reserva e feita em duas camadas: as faixas sao marcadas no bitmap do alocador de paginas fisicas, para nunca serem entregues como RAM comum, e tambem tem o bit `PRESENT` removido das tabelas de pagina, com `invlpg`. Assim, qualquer acesso a elas gera page fault. O LAPIC e o IOAPIC sao desmapeados e depois remapeados por `nucleo/cpu/apic.rs` e `nucleo/cpu/ioapic.rs` como dispositivos nao cacheaveis, pois o nucleo precisa acessa-los. O HPET ainda nao e programado: a regiao apenas e reservada e protegida.

O núcleo inicializa um alocador de páginas físicas usando apenas memória convencional UEFI, reserva as áreas do núcleo/framebuffer/dados de boot e as áreas de hardware, e instala um mapeamento identidade dos primeiros 64 GiB. Código do núcleo fica somente leitura/executável; dados ficam leitura/escrita/não executáveis; o restante do mapeamento fica não executável. NX e proteção de escrita (CR0.WP) são habilitados, e o framebuffer é mapeado como não cacheável. O heap global usa páginas físicas e executa um teste de alocação na inicialização. O alocador gerencia endereços físicos abaixo de 64 GiB. Ainda não há processos de usuário nem separação de espaços de endereçamento por processo. Arquivos permanecem no armazenamento e seus dados são carregados na RAM quando usados. A listagem mostra endereços e tipos UEFI; as linhas rolam a tela quando necessário. Todas as linhas, de qualquer etapa, são desenhadas com uma pausa de 25 ms entre si, para que apareçam uma por uma.

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

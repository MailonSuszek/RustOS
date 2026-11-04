# RustOS Beta

**Arquivo:** README.md<br>
**Funcao:** Documenta a estrutura, os requisitos, a compilacao e a execucao do RustOS.<br>
**Nome:** RustOS Beta  
**Versao:** v0.1  
**Construcao:** 0.11.2026<br>
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
- `nucleo/cpu/pat.rs`: configura as oito entradas PAT (`WB, WT, UC-, UC, WC, WP, UC-, UC`) quando o processador oferece suporte.
- `nucleo/segurança/quando.rs`: orquestra em ordem as etapas de exibicao do nucleo para evitar execucao fora de sequencia.
- `nucleo/segurança/o_que.rs`: inicializa a memoria pertencente ao nucleo e exibe o estado de protecao apos a inicializacao bem-sucedida.
- `nucleo/segurança/aleatorio.rs`: inicializa um CRNG ChaCha20 com entropia do protocolo RNG UEFI, das instrucoes RDSEED/RDRAND ou do TPM 2.0 via TCG2.
- `nucleo/segurança/secureboot.rs`: exibe se o Secure Boot estava ativo ou desativado quando o carregador UEFI entregou o controle ao nucleo.
- `nucleo/memoria/enderecos.rs`: interpreta o mapa UEFI e lista os intervalos de RAM.
- `nucleo/memoria/paginas.rs`: aloca, reserva e libera paginas fisicas entre as regioes UEFI convencionais, com limite de 64 GiB.
- `nucleo/memoria/ponteiros.rs`: exibe os enderecos de tabelas UEFI recebidos pelo carregador, incluindo ACPI e SMBIOS.
- `nucleo/memoria/regioes.rs`: reserva explicitamente a primeira pagina fisica (0-4 KiB), a imagem do nucleo, o framebuffer e os dados de boot.
- `nucleo/memoria/mapa_memoria.rs`: tabela unica das areas de hardware (video, ROM da BIOS, chipset, IOAPIC, HPET, Local APIC e Flash da BIOS); reserva essas faixas no alocador fisico, remove o mapeamento delas e gera as linhas correspondentes na tela.
- `nucleo/memoria/mtrr.rs`: verifica se a RAM convencional usa write-back e configura write-combining no framebuffer apenas quando há faixa MTRR livre e sem sobreposição.
- `nucleo/memoria/virtual.rs`: cria um mapeamento identidade de 64 GiB com páginas de 1 GiB quando suportadas (fallback para páginas de 2 MiB), protege as secoes do nucleo, habilita NX e CR0.WP, seleciona PAT WC no framebuffer somente quando MTRR WC está ativo (senão mantém UC) e desmapeia as reservas.
- `nucleo/memoria/bloco.rs`: cria um bloco virtual unico em 1 TiB que espelha toda a RAM fisica com paginas grandes de 2 MiB, reserva uma pagina fisica para a estrutura de controle do bloco e exibe as duas linhas com a faixa virtual criada e o endereço da configuracao.
- `nucleo/memoria/zonas.rs`: divide a RAM em zonas de hardware no padrao DMA (0-16 MiB), DMA32 (16 MiB-4 GiB) e Normal (acima de 4 GiB) e escreve uma linha para cada zona, informando faixa e tamanho ou indicando que a zona fica vazia na maquina.
- `nucleo/memoria/zona_movel.rs`: reserva a metade superior da RAM como zona movel para dados que podem ser realocados facilmente, mantendo a area contigua contra fragmentacao, e escreve a linha de inicio da zona movel.
- `nucleo/memoria/memoria_dinamica.rs`: fornece o alocador global do nucleo baseado em paginas fisicas.
- `nucleo/memoria/operacao.rs`: confirma pelos dados SMBIOS se todos os slots e canais de memoria estao ocupados.
- `nucleo/Rust`: executavel ELF independente, produzido a partir de `nucleo/nucleo.rs` e incluido na pasta `NUCLEO` da ISO 9660.
- `nucleo/info/uefi.rs`: exibe a versao do UEFI obtida de `SystemTable.Header.Revision` no carregador.
- `nucleo/info/smbios.rs`: exibe a versao do SMBIOS capturada pelo carregador a partir da entrada `_SM3_` ou `_SM_` da tabela de configuracao UEFI.
- `nucleo/info/dmi.rs`: exibe fabricante, modelo e BIOS do computador obtidos das estruturas DMI/SMBIOS capturadas pelo carregador.
- `nucleo/info/acpi.rs`: caminha o RSDP recebido do UEFI, valida a XSDT (ou RSDT) e lista cada tabela ACPI valida em uma linha, com assinatura, endereço, tamanho e fabricante.
- `nucleo/linker.ld`: posiciona o nucleo ELF na memoria fisica a partir de 1 MiB.
- `inicio/inicio.img`: imagem FAT de boot que contem `EFI/BOOT/BOOTX64.EFI` e `inicio.so` na raiz.
- `compilar.sh`: compila os estagios UEFI e o nucleo freestanding, e gera a imagem FAT e a ISO.
- `.vscode/tasks.json`: tarefas para compilar e executar no QEMU.
- `RustOS_Beta_v0.1_Construcao_0.11.2026.iso`: imagem ISO gerada na raiz do projeto.

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
{   1} RustOS Beta v0.1 Construção 0.11.2026
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
{próximo número} 0000000000000000 - 0000000000000FFF (4 KB) Endereço reservado
{próximo número} 00000000000A0000 - 00000000000FFFFF (384 KB) Endereço reservado
{próximo número} RAM disponível: X.XX GB
{próximo número} Memória: RAM WB; framebuffer WC
{próximo número} PAT: WB, WT, UC-, UC, WC, WP, UC-, UC
{próximo número} RAM fisica utilizavel: X.XX GB
{próximo número} Paginas grandes ativas: 1 GiB (PDPE1GB)
{próximo número} ramdisk não encontrado
{próximo número} Tabelas ACPI disponibilizadas pela UEFI: N
{próximo número} Tabela ACPI: FACP, endereço 0x00000000000E0000, 268 bytes, fabricante BOCHS
{próximo número} Bloco virtual de RAM criado: 0x0000001000000000 - 0x0000001010000000
{próximo número} Controle do bloco reservado: 0x0000000000012A000
{próximo número} Zona DMA: 0x0000000000000000 - 0x0000000000FFFFFF (16 MB)
{próximo número} Zona DMA32: 0x0000000001000000 - 0x000000000FFFFFFF (240 MB)
{próximo número} Zona Normal: vazia
{próximo número} Início da zona móvel para a memoria RAM
```

O número da versão EFI e obtido de `SystemTable.Header.Revision`; o exemplo `2.70` pode variar conforme o firmware. Depois dessa linha, o carregador passa ao nucleo os enderecos das tabelas UEFI, identificando ACPI e SMBIOS quando reconhecidos. Todas as linhas recebem numeracao sequencial, uma por uma, inclusive as de continuacao quando a lista de tabelas nao cabe em uma unica linha. O nucleo tambem usa a MADT ACPI para localizar os IOAPICs. O CRNG usa ChaCha20 com uma semente de 256 bits obtida do protocolo RNG do UEFI, de RDSEED/RDRAND ou de TPM 2.0 via protocolo TCG2; sem uma fonte disponivel, a inicializacao falha em vez de usar uma semente previsivel.

As ultimas linhas seguem esta ordem: frequencia do TSC, primeira pagina reservada, faixa combinada de video/BIOS, RAM disponivel, estado MTRR, oito tipos PAT, total de RAM fisica utilizavel, tamanho das paginas grandes ativas, estado do ramdisk, a listagem das tabelas ACPI, o bloco virtual de RAM, as zonas de RAM e a zona movel. A mensagem de páginas grandes informa 1 GiB quando `PDPE1GB` e suportado; caso contrario, informa o fallback de 2 MiB. Como a imagem do ramdisk ainda nao existe, a linha do ramdisk mostra apenas `ramdisk não encontrado`; o endereço real será exibido quando ele for carregado. A linha de RAM fisica soma os descritores UEFI de memoria convencional limitados ao alcance de 64 GiB do alocador RustOS. `nucleo/cpu/tsc.rs` calibra o TSC contra o PIT por cerca de 27 ms e exibe a velocidade detectada; se o PIT nao responder, a frequencia e obtida pelas folhas CPUID `0x15` e `0x16`.

Por fim, o nucleo identifica as tabelas ACPI disponibilizadas pela UEFI pelo RSDP entregue pelo carregador: valida a assinatura e o checksum do RSDT/XSDT, preferindo a XSDT de 64 bits, e percorre cada entrada. Cada tabela com cabecalho e checksum validos aparece em uma propria linha no fim da tela, com assinatura, endereço fisico, tamanho e fabricante, na ordem do firmware; tabelas invalidas sao ignoradas. Quando nao ha RSDP a linha informa `Tabelas ACPI: RSDP ausente`, e quando nenhuma tabela e valida a linha informa `Tabelas ACPI: nenhuma encontrada`.

Depois da listagem ACPI, o nucleo cria um bloco virtual unico para gerenciar toda a RAM do computador: calcula o fim da RAM pelo mapa UEFI ignorando as regioes reservadas e de dispositivo mapeado, limita o resultado a 64 GiB e arredonda para cima em 2 MiB, e entao mapeia a faixa fisica `0` ate esse fim em um endereco virtual dedicado de 1 TiB (`0x0000001000000000`) com paginas grandes de 2 MiB, sem alterar o mapa identidade. Em seguida o nucleo reserva uma pagina fisica via alocador onde grava a estrutura de controle do bloco (base virtual, base fisica, tamanho e numero de paginas), que e o endereco de configuracao mostrado na segunda linha. As duas ultimas linhas da tela informam a faixa virtual criada e o endereço da estrutura de controle; se a RAM nao for encontrada, o mapeamento falhar ou a pagina de controle nao puder ser alocada, a inicializacao falha com a mensagem `Erro bloco memoria`.

Por fim, a RAM e dividida em zonas de hardware no padrao DMA/DMA32/Normal: a Zona DMA cobre de 0 ate 16 MiB para dispositivos com enderecamento limitado, a Zona DMA32 cobre de 16 MiB ate 4 GiB para dispositivos de 32 bits e a Zona Normal cobre o que restar acima de 4 GiB para dispositivos de 64 bits. Cada zona recebe uma propria linha no fim da tela com a faixa fisica e o tamanho em MB, limitadas ao fim da RAM calculado para a maquina; as zonas que nao possuem espaco na RAM aparecem com a palavra `vazia`, de modo que as tres linhas sao sempre escritas.

Apos as zonas, o nucleo prepara a zona movel: a metade superior da RAM, do meio ate o fim calculado, e reservada no alocador de paginas para conter dados moveis, que podem ser realocados de lugar com facilidade. Manter essa area contigua e livre de alocacoes comuns evita a fragmentacao da memoria e abre caminho para funcoes avancadas, como a hibernacao do sistema no futuro. As paginas da faixa sao marcadas como indisponiveis para a RAM comum, sem erro quando ja estiverem ocupadas por kernel ou ACPI, e a ultima linha da tela informa `Início da zona móvel para a memoria RAM`.

O modulo MTRR exige suporte a MTRR e PAT, confirma que as regioes UEFI de RAM convencional permanecem write-back e só usa registradores variáveis livres para marcar o framebuffer como write-combining. Se a faixa se sobrepõe a RAM ou a outro MTRR, não há registradores livres ou as condições de segurança não são satisfeitas, as regras do firmware são preservadas e o framebuffer permanece não cacheável.

O modulo PAT escreve as oito entradas na ordem exibida. Se PAT ou MTRR nao forem suportados ou as verificacoes MTRR falharem, o framebuffer continua UC e a linha PAT informa que a tabela do firmware foi preservada quando PAT nao puder ser configurada.

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
| Faixa combinada de vídeo e BIOS | `0x000A0000 - 0x000FFFFF` | 384 KB |

Os tamanhos sao calculados a partir da propria faixa (`fim - inicio + 1`), nao digitados. A tabela `REGIOES` em `nucleo/memoria/mapa_memoria.rs` e a unica fonte para reserva fisica, desmapeamento e texto na tela, evitando divergencia entre o que e exibido e o que e feito. O IOAPIC e uma sub-regiao do bloco de controle do chipset, como no mapa real de `/proc/iomem` do Linux.

A reserva e feita em duas camadas: as faixas sao marcadas no bitmap do alocador de paginas fisicas, para nunca serem entregues como RAM comum, e tambem tem o bit `PRESENT` removido das tabelas de pagina, com `invlpg`. Assim, qualquer acesso a elas gera page fault. O LAPIC e o IOAPIC sao desmapeados e depois remapeados por `nucleo/cpu/apic.rs` e `nucleo/cpu/ioapic.rs` como dispositivos nao cacheaveis, pois o nucleo precisa acessa-los. O HPET ainda nao e programado: a regiao apenas e reservada e protegida.

A primeira pagina fisica (`0x0-0xFFF`) tambem e marcada como indisponivel no bitmap e removida do mapeamento identidade. Assim, ela nao pode ser entregue pelo alocador nem acessada pelo nucleo por meio desse mapeamento.

A linha `RAM disponível` conta as páginas livres no bitmap real do alocador após as reservas do kernel e das regiões de hardware. A linha final soma a memoria fisica convencional descrita pelo UEFI dentro do limite do alocador; ambas convertem bytes para GB decimal (1 GB = 1.000.000.000 bytes) e arredondam para duas casas decimais.

O núcleo inicializa um alocador de páginas físicas usando apenas memória convencional UEFI, reserva as áreas do núcleo/framebuffer/dados de boot e as áreas de hardware, e instala um mapeamento identidade dos primeiros 64 GiB. Código do núcleo fica somente leitura/executável; dados ficam leitura/escrita/não executáveis; o restante do mapeamento fica não executável. NX e proteção de escrita (CR0.WP) são habilitados. O framebuffer recebe WC quando as verificações de PAT/MTRR passam; caso contrário, permanece UC. O heap global usa páginas físicas e executa um teste de alocação na inicialização. O alocador gerencia endereços físicos abaixo de 64 GiB. Ainda não há processos de usuário nem separação de espaços de endereçamento por processo. Arquivos permanecem no armazenamento e seus dados são carregados na RAM quando usados. A listagem mostra endereços e tipos UEFI; as linhas rolam a tela quando necessário. Todas as linhas, de qualquer etapa, são desenhadas com uma pausa de 25 ms entre si, para que apareçam uma por uma.

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

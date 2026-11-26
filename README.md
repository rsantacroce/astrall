# Astrall

<img src="site/img/icon-256.png" width="96" alt="Ícone do Astrall" align="right" />

**Site e downloads: <https://rsantacroce.github.io/astrall/>**

Mapa astral natal em um app de desktop e iOS (Rust + Tauri 2 + Svelte). O app faz a entrevista
(nome, data, horário, cidade, temas, opções), mostra a conferência de fuso e horário de verão,
calcula o mapa no próprio aparelho e guarda cada mapa e cada leitura no histórico.

## Como está dividido

| Parte | Onde | O que faz |
| --- | --- | --- |
| Cálculo | `crates/astro-core` | Posições, casas, aspectos, destaques e trânsitos. Rust puro, sem Tauri. |
| App | `src-tauri` | Cidades e fusos offline, SQLite, chaves no cofre do sistema, provedores de LLM. |
| Interface | `src` | Svelte 5 + TypeScript: entrevista, roda do mapa, tabelas, trânsitos, leituras. |

O cálculo nunca depende de internet nem de modelo de linguagem. O LLM só escreve a interpretação,
a partir das posições já calculadas (o prompt está em `src-tauri/src/prompt.rs`). Sem provedor
configurado dá para copiar o prompt pronto, usar em qualquer chat e colar a resposta de volta.

## Precisão

- Sol e planetas: VSOP87; Lua: ELP-2000/82 truncada (crate `astro`, MIT).
- Plutão e Quíron: posições heliocêntricas do JPL Horizons embutidas e interpoladas (1800–2199).
- `cargo test` compara tudo com o JPL Horizons: até 2024 o erro fica abaixo de 2″ nos planetas e
  de 6″ na Lua; o Nodo verdadeiro (osculador) confere dentro de 2′.
- Casas: Placidus, Regiomontanus, Porphyry, iguais e signos inteiros. Koch e Campanus ainda não.
- Fuso e horário de verão históricos: banco IANA (`chrono-tz`). Cidades: GeoNames, 172 mil lugares.

## Motores de efemérides

O mapa e os trânsitos falam com as efemérides só pelo trait `Ephemeris`
(`crates/astro-core/src/engine.rs`); o motor é escolhido por mapa.

| Motor | Como entra | Licença |
| --- | --- | --- |
| Nativo | sempre | sem restrição própria (crate `astro` MIT, dados do JPL em domínio público) |
| Swiss Ephemeris | `--features swisseph` | AGPL-3.0, ou licença comercial da Astrodienst |

```sh
npm run tauri dev -- --features swisseph
npm run tauri build -- --features swisseph
cargo test -p astro-core --features swisseph   # confronta os dois motores
```

O build padrão não compila nem linka a Swiss Ephemeris. Com o recurso
ligado, a opção "Motor de efemérides" aparece na entrevista. Sem arquivos de dados a biblioteca
usa o Moshier embutido e Quíron vem do motor nativo (o mapa avisa); com os `.se1` (`sepl_18`,
`semo_18`, `seas_18`) na pasta `ephe` dos dados do app ela usa as efemérides completas.

Um motor novo é um tipo que implementa `Ephemeris` mais uma linha em `engine::get`.

## Desenvolvimento

```sh
npm install
npm run tauri dev        # app de desktop
npm test                 # testes do cálculo e do backend
npm run tauri build      # instalador da plataforma atual
```

iOS (precisa de Xcode e `brew install xcodegen`):

```sh
npm run tauri ios init   # gera src-tauri/gen/apple (uma vez)
npm run tauri ios dev    # simulador ou aparelho
```

Para rodar num aparelho ou gerar o `.ipa`, defina o time de desenvolvimento em
`bundle.iOS.developmentTeam` no `tauri.conf.json` ou na variável `APPLE_DEVELOPMENT_TEAM`.

## Builds para todas as plataformas

`.github/workflows/release.yml` roda ao enviar uma tag `v*`, gera macOS (universal), Windows
(x64 e ARM64) e Linux (x64 e ARM64) e publica a release com os instaladores. A landing page
(`site/`, publicada por `pages.yml`) lê a release mais recente e aponta os botões de download
para ela. O job de iOS é experimental e não bloqueia a release.
Assinatura é opcional e vem de segredos do repositório:

- macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`,
  `APPLE_PASSWORD`, `APPLE_TEAM_ID`.
- iOS: `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_CONTENT`, `APPLE_TEAM_ID`. Sem eles o
  job só confirma que o app compila para o simulador.

## Dados embutidos

- `scripts/gen_ephem_tables.py`: refaz as tabelas de Plutão/Quíron e a referência dos testes (JPL Horizons).
- `scripts/build_cities.py`: refaz a base de cidades (GeoNames, CC BY 4.0).

## Onde ficam os dados

Banco SQLite em `astrall.db`, na pasta de dados do app (no macOS,
`~/Library/Application Support/xyz.santacroce.astrall/`). As chaves de API ficam no cofre do sistema.

## Licença

AGPL-3.0 (veja `LICENSE`). Copyright © 2026 Roberto Santacroce Martins.

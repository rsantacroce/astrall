# Astrall: decisões e próximos passos

Anotações das conversas de 26/11/2026. O que já existe está no `README.md`; aqui fica o que foi
decidido, o que foi só discutido e a ordem sugerida.

## Decisões tomadas

- **Cálculo separado da interpretação.** As posições são calculadas em Rust, offline; o modelo de
  linguagem só escreve o texto a partir dos números prontos.
- **Efemérides em camada.** Motor nativo (licença livre) por padrão; Swiss Ephemeris opcional atrás
  do recurso de compilação `swisseph` (AGPL), usada também como gabarito nos testes.
- **Plataformas.** macOS, Windows, Linux e iOS. Android depois.
- **LLM local primeiro.** O provedor padrão é o Ollama local com modelo quantizado. Claude, OpenAI
  e APIs compatíveis continuam disponíveis nas configurações.

## Em aberto, na ordem sugerida

### 1. Medir modelos locais prontos

Rodar dois ou três modelos quantizados (famílias Gemma, Qwen, Llama, Phi) no Ollama, com o prompt
atual, em cerca de vinte mapas. Medir:

- **Fidelidade**: cada posicionamento citado na leitura existe no mapa? Dá para verificar por código.
- **Texto**: qualidade do português e respeito à estrutura, em comparação às cegas.

Falta instalar o Ollama (`brew install ollama`) e baixar os modelos. Este passo decide os
passos 3, 4 e 5.

### 2. Leitura resumida e chat por mapa

- Seletor de profundidade (resumo de uma tela ou leitura completa), resolvido no prompt.
- Uma conversa por mapa, salva no histórico; cada pergunta leva junto as posições calculadas.

### 3. Base de textos por posicionamento

Textos curtos nossos para cada posicionamento (Sol em Leão, Lua na casa 9, Marte quadratura
Netuno). O app seleciona os trechos do mapa e o modelo só costura e personaliza. Motivo: modelo
pequeno reescreve bem e "sabe" mal. Vale se o passo 1 mostrar erros de significado.

### 4. Modelo embutido no app

Rodar o modelo dentro do app, sem Ollama; é o que permite leitura offline no iPhone. Entra como um
terceiro tipo de provedor ("embutido").

| Opção | Formato | A favor | Contra |
| --- | --- | --- | --- |
| llama.cpp (bindings Rust) | GGUF | o mais maduro, mesmo formato do Ollama | C++ para compilar em cada plataforma |
| candle (Hugging Face) | GGUF e outros | Rust puro | menos maduro e otimizado |
| ONNX Runtime (crate `ort`) | ONNX | boa aceleração da Apple no iOS | menos modelos prontos, conversão a mais |

Recomendação: GGUF, começando por llama.cpp, porque o mesmo arquivo serve no Ollama do desktop e
embutido no iOS. Nenhuma das três foi testada neste projeto; conferir a documentação atual antes.

JEPA não se aplica: é arquitetura de representação, não gera texto.

### 5. Ajuste fino de um modelo pequeno

Só se o passo 1 mostrar que o modelo pronto não segura o formato ou inventa dados.

- **O que é**: LoRA sobre um modelo aberto de 1 a 4 bilhões de parâmetros; quantizado em 4 bits fica
  em 1 a 3 GB. Entrada: o mapa no formato que o app já gera. Saída: a leitura na nossa estrutura.
- **O que não é**: um modelo treinado do zero só com astrologia. A fluência em português vem do
  pré-treino geral; o modelo fica leve pela base escolhida, não por remover conhecimento.
- **Ganhos**: formato e tom fixos sem prompt longo, menos dados inventados, tamanho viável para iOS.
- **Dados**: os mapas vêm do nosso motor, de graça. As leituras vêm de destilação (um modelo grande
  escreve, a gente filtra) ou de texto humano escrito ou licenciado. Antes de destilar de um
  provedor comercial, conferir se os termos permitem treinar outro modelo com as saídas; um
  professor de pesos abertos evita a questão.

### 6. Motor de efemérides próprio

Terceiro motor atrás do trait `Ephemeris`, substituindo o crate `astro` e as tabelas do Horizons:
tabelas nossas geradas a partir das efemérides DE440 do JPL (domínio público) para todos os corpos,
com precessão e nutação nossas.

- **Ganho**: a Lua cai de 6″ para bem abaixo de 1″; nenhuma dependência astronômica de terceiros.
- **Custo**: alguns megabytes de dados embutidos (quase tudo pela Lua) e alguns dias de trabalho.
- **Risco**: baixo; a Swiss Ephemeris serve de gabarito.

### 7. Pendências menores

- Casas Koch e Campanus (hoje só Placidus, Regiomontanus, Porphyry, iguais e signos inteiros).
- Exportar e importar o histórico.
- Android.

## Ainda não verificado

- Geração de leitura contra um provedor real (não havia chave nem Ollama na máquina).
- Build do iOS no Xcode (falta instalar a plataforma iOS no Xcode 27.2).
- Workflows de CI e release; builds de Windows e Linux.
- Instalador `.dmg` local (o Finder não respondeu ao AppleScript na sessão de build).
- Assinatura da Apple e arquivo de licença do projeto.

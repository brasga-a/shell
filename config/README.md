# Configuração de desenvolvimento

Esta pasta espelha a estrutura modular aceita pelo shell. Em runtime, o
loader procura primeiro `ATLANTIC_CONFIG_DIR`, depois
`$XDG_CONFIG_HOME/atlantic` e, por fim, `~/.config/atlantic`.

Os arquivos daqui podem ser usados localmente com:

```sh
ATLANTIC_CONFIG_DIR=./config cargo run --bin shell-app
```

Arquivos ausentes recebem os defaults seguros do modelo. Arquivos de módulos
desconhecidos são ignorados com diagnóstico.

Quando o shell é executado com `--gpui-viability`, a árvore inteira é
monitorada recursivamente. Alterações são agrupadas por uma janela de 200 ms,
carregadas como um snapshot completo e aplicadas somente depois da validação.
Se o TOML estiver temporariamente inválido, o snapshot anterior continua ativo.

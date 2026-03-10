# recap

コマンドの実行結果を自動で保存・検索できる CLI ツール。
MCP サーバー機能を内蔵しており、AI エージェントから過去の実行履歴を参照できます。

## インストール

```bash
cargo install --path .
```

## セットアップ

シェルの設定ファイルに以下を追加します。

**bash** (`~/.bashrc`):
```bash
eval "$(recap init bash)"
```

**zsh** (`~/.zshrc`):
```bash
eval "$(recap init zsh)"
```

**fish** (`~/.config/fish/config.fish`):
```fish
recap init fish | source
```

これにより `rec` 関数が定義されます。

## 使い方

### コマンドの実行と記録

```bash
rec "ls -la"
rec "cargo test"
rec "curl -s https://example.com | jq ."
```

`rec` で実行したコマンドは、出力・終了コード・タイムスタンプが自動的に `~/.recap/` に保存されます。

### 履歴の確認

```bash
# 直前の実行結果を表示
recap -l

# 直近 N 件の一覧
recap -n 20

# 指定 ID の実行結果を表示
recap -s 3
```

## MCP サーバー

recap は [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) サーバーとして動作し、AI エージェントにコマンド実行履歴を提供できます。

### Claude Code での設定

```json
{
  "mcpServers": {
    "recap": {
      "command": "recap",
      "args": ["--mcp"]
    }
  }
}
```

### 提供ツール

| ツール | 説明 |
|---|---|
| `recap_last` | 直前のコマンド実行結果を取得 |
| `recap_list` | 実行履歴の一覧を取得 (limit で件数指定) |
| `recap_get` | 指定 ID の実行結果を取得 |

## データ保存先

```
~/.recap/
├── index.jsonl   # 実行履歴のインデックス (JSONL)
└── logs/
    ├── 0001.log  # 各コマンドの出力
    ├── 0002.log
    └── ...
```

## ライセンス

MIT

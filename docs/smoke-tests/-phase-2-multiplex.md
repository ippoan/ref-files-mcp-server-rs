# Phase 2-5 受け入れ検定: 2 binary 多重 attach smoke test

Issue: [ref-files-mcp-server-rs#8](https://github.com/ippoan/ref-files-mcp-server-rs/issues/8) (Refs ref-files-mcp-server-rs#4 Phase 2-5)

`github-mcp-server-rs` + `ref-files-mcp-server-rs` の 2 binary を同一 CCoW container で
並行 attach し、auth-worker mcp-relay-bridge が両方の JWT `aud` claim を accept して
**同一 `tools/list` に両 binary の tool が出ること** を実機確認する。

## 前提 (Phase 2-1 〜 2-4)

実行前に下記が全て完了していること:

- [x] ippoan/mcp-relay-rs bootstrap (`dev-1` tag push 済)
- [x] ippoan/github-mcp-server-rs#61 (Phase 2-2 refactor) merged
- [x] ippoan/ref-files-mcp-server-rs#5 (Phase 2-3 integration) merged
- [x] ippoan/auth-worker#167 (Phase 2-4 multiplex option C) merged
- [ ] ippoan/auth-worker#170 (`MCP_JWT_AUDIENCE_ALLOWLIST` [vars] 追加) merged + staging deploy 完了
- [ ] `ref-files-mcp-server-rs` の `v0.0.1` (or `dev-N`) tag が linux x86_64 tarball + sha256 を release に attach 済

最後の 2 つが揃ったら以下に進む。

## 実行環境

| 項目 | 値 |
|---|---|
| date (UTC) | _YYYY-MM-DD HH:MM_ |
| tester | _@github-login_ |
| CCoW container | _session URL or container ID_ |
| auth-worker target | `https://mcp-staging.ippoan.org` |
| `GITHUB_LOGIN` env | _your github login_ |
| ref-files-mcp version | _tag / sha_ |
| github-mcp-server version | _tag / sha_ |

## Step 1: 2 binary install

```bash
# CCoW container 内 (両 install-mcp.sh は dev-N or v0.0.X tag から binary を取得)
export GITHUB_LOGIN=<your-github-login>

curl -fsSL https://raw.githubusercontent.com/ippoan/ref-files-mcp-server-rs/main/install-mcp.sh | bash
curl -fsSL https://raw.githubusercontent.com/ippoan/github-mcp-server-rs/main/install-mcp.sh | bash
```

実行ログ・成功確認:

- [ ] ref-files-mcp-server-rs binary 設置完了 (path: _________)
- [ ] github-mcp-server-rs binary 設置完了 (path: _________)
- [ ] それぞれ device flow で JWT 取得し `~/.config/mcp/<binary>/token` 等に保存 (path: _________)

## Step 2: Claude Code Web から接続

Claude Code Web の MCP servers 設定で `https://mcp-staging.ippoan.org/u/<GITHUB_LOGIN>/mcp` を **1 つだけ** 登録 (multiplex は 1 endpoint で両 binary 集約)。

接続確認:

- [ ] `tools/list` レスポンスに **ref-files 系** (`repo_init`, `folder_create`, `folder_delete`, `file_*` ...) が含まれる
- [ ] 同じ `tools/list` に **github 系** (`admin_exec_*`, `whoami`, repo / issue / PR 系) が含まれる
- [ ] tool 名衝突 fail-fast (auth-worker `mcp-session-do.ts` `aggregateToolsList`) が `tool name conflict` を返していないこと (= 名前衝突無し)

`tools/list` レスポンス抜粋 (両 binary の tool が並んでいる行を記録):

```jsonc
// paste tools[] array here, redacting any sensitive metadata
```

## Step 3: tools/call 実機叩き

ref-files-worker D1 に新規 row を作る + github_login が引ける、両方が同一 session で成功すること。

```jsonc
// tools/call repo_init
{ "name": "repo_init", "arguments": { ... } }
```

- [ ] `repo_init` 成功 → ref-files-worker D1 に row が出来た (worker GET で確認: _________)
- [ ] `whoami` 成功 → 返却された github_login が `GITHUB_LOGIN` env と一致

レスポンス抜粋:

```jsonc
// paste responses here
```

## Step 4: Hibernation 跨ぎ permanence

Hibernatable WebSocket 仕様上、`ws.serializeAttachment` で保存した state は hibernation
sleep / wake を跨いで保持されるが、Phase 2 multiplex で 2 binary の attachment が独立に
保持されることを 1 回実機確認する。

手順:

1. 上記で接続済の状態を 5〜15 分放置 (DO が hibernate に入る window)
2. Claude Code Web からもう一度 `tools/list` を叩く
3. 再 attach 無しで 2 binary の tool が依然出ること

- [ ] hibernation 後の `tools/list` で両 binary の tool が引き続き返る (= attachment 永続確認)

## Step 5: 結果サマリ

| Step | Pass / Fail | 備考 |
|---|---|---|
| 1 install | ☐ Pass / ☐ Fail |  |
| 2 接続 + tools/list | ☐ Pass / ☐ Fail |  |
| 3 tools/call (`repo_init` / `whoami`) | ☐ Pass / ☐ Fail |  |
| 4 hibernation permanence | ☐ Pass / ☐ Fail |  |

**全 Pass で Phase 2 完走宣言** → consumer Cargo.toml の `tag = "dev-N"` を必要なら latest dev-N に bump、または stable 昇格を別途検討。

## ありそうな落とし穴 (Phase 2 特有、issue #8 から)

- `tools.listChanged: false` が auth-worker で有効 → binary が後 attach しても Claude Code Web に push 更新が来ない。**MCP entry の 「切断 → 再接続」が必要**
- tool 名衝突は fail-fast (auth-worker `mcp-session-do.ts` `aggregateToolsList`) で `error: tool name conflict between services` を返す → 出たら衝突 tool 名を記録して別途 fix issue を立てる
- `ref-files-mcp-server-rs` の JWT が relay-bridge で 401 → `MCP_JWT_AUDIENCE_ALLOWLIST` 未反映 (#170 staging deploy 待ち)

## 関連

- ippoan/ref-files-mcp-server-rs#4 — Phase 2 master issue
- ippoan/auth-worker#167 — Phase 2-4 multiplex option C
- ippoan/auth-worker#170 — `MCP_JWT_AUDIENCE_ALLOWLIST` [vars] 追加 (前段 blocker)
- ippoan/github-mcp-server-rs#61 — Phase 2-2 refactor
- ippoan/ref-files-mcp-server-rs#5 — Phase 2-3 integration

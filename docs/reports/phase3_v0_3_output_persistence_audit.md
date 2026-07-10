# Phase 3.4 v0.3 Output Persistence Audit

## Summary

v0.3 预定的用户功能与生产路径已经就绪：storage 提供原子的
`CreateNew` 执行边界，CLI 通过显式 `--output <path>` 保存确定性 JSON，
默认命令仍不写文件，现有目标不会被覆盖，并发回归测试可证明两个写入者中
恰有一个成功。CLI、doctor 和 prompt kernel 版本均为 `0.3.0`。

全部允许的格式、编译、库测试、storage integration tests、CLI 进程测试和
命令输出验证均通过。原审计发现的测试隔离 blocker 已由 PR #31（`e0908bc`）
消除：storage 文件系统测试已从 `atomic.rs` 的 unit-test 模块迁移到现有
integration test target，并统一写入 `env!("CARGO_TARGET_TMPDIR")` 下的
独立精确子目录。该修复只移动测试，未改变 production storage 实现、公共
API 或用户行为。

- Functional readiness: **yes**
- Release/tag readiness: **yes**
- `v0.3.0` tagging recommendation: **本报告审阅并提交后推荐创建 annotated tag**

## Commands Run

- `git status --short`
- `git branch --show-current`
- `git log --oneline --decorate -n 8`
- `git tag --list "v0.3.0"`
- `git ls-remote --tags origin v0.3.0`
- 只读 `wc`、`sed` 和 `rg` 检查指定文档、报告、源码、测试和路径隔离
- `rg -n "std::env::temp_dir|env::temp_dir|temp_dir\\(" crates/deepseek-science-storage`
- `cargo fmt --check`
- `cargo check -p deepseek-science-storage`
- `cargo test -p deepseek-science-storage --lib`
- `cargo test -p deepseek-science-storage --test atomic_create_new`
- `cargo check -p deepseek-science-cli`
- `cargo test -p deepseek-science-cli --lib`
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`
- `cargo run -p deepseek-science-cli -- version`
- `cargo run -p deepseek-science-cli -- doctor`
- `cargo run -p deepseek-science-cli -- kinetics analyze --help`
- `cargo check --workspace`
- `cargo test --workspace --lib`
- `git diff --no-index -- /dev/null docs/reports/phase3_v0_3_output_persistence_audit.md || true`
- `git status --short`

本次 refresh 中每个指定 Cargo 命令只运行一次。自动化测试已经覆盖所需输出
和失败路径，因此未额外创建手工 smoke 目录或文件。报告更新后未再运行
Cargo 命令。

## Git Status

- 当前分支为 `main`。
- refresh 开始时唯一工作区条目是预期的未跟踪审计报告，没有 source-tree
  build output、tracked change 或意外未跟踪文件。
- `HEAD`、本地 `main`、`origin/main` 和 `origin/HEAD` 均指向
  `e0908bc`：`test: isolate atomic writes under cargo target (#31)`。
- 历史包含 Phase 3 的 PR #27、#28、#29、#30 和 blocker 修复 PR #31。
- 本地 `git tag --list "v0.3.0"` 无输出。
- 远端 `git ls-remote --tags origin v0.3.0` 无输出。
- 本次任务未创建 tag 或 GitHub Release。
- 报告完成后的唯一工作区变化应为本报告。

## Version Status

- `crates/deepseek-science-cli/Cargo.toml` 的包版本为 `0.3.0`。
- `deepseek-science version` 输出 `deepseek-science 0.3.0`。
- `deepseek-science doctor` 输出 `version: 0.3.0` 和
  `prompt_kernel_version: 0.3.0`，并以 `status: ok` 结束。
- `version`、`doctor` 的实际输出中没有用户可见的 `0.2.0`。
- `version`、doctor 版本和 `PromptVersionInfo` 都读取
  `env!("CARGO_PKG_VERSION")`，没有第二个硬编码版本源。
- 初始审计的 `cargo tree --workspace` 将 CLI 报告为
  `deepseek-science-cli v0.3.0`；PR #31 没有修改依赖 metadata。
- 仓库没有统一提升所有 workspace crate 版本的策略；其他内部 crate 保持
  `0.1.0` 不构成 v0.3 阻塞项。

## Default No-write Status

- `KineticsAnalyzeArgs.output_path` 默认为 `None`。
- `analyze_kinetics_csv` 只在 `output_path` 为 `Some` 时调用
  `write_json_output_file`；默认文本和仅 `--json` 路径不进入 storage
  执行边界。
- 文本仍是默认 stdout 格式；`--json` 只改变成功 stdout 格式。
- CLI 成功输出只在 `analyze_kinetics_csv` 全部完成后构造为
  `CliOutput::success`；错误为简洁的人类可读 stderr，stdout 为空。
- 默认分析仅读取一个显式输入文件，不创建 output、workspace、artifact、
  run record、log、cache 或 temp directory。
- CLI 进程测试继续覆盖无 `--output` 的文本和 JSON 成功路径。

## Output File Behavior

- `--output <path>` 在分析成功后保存一份确定性 UTF-8 JSON 文件。
- 没有 `--json` 时，stdout 保持人类可读文本，目标文件接收 JSON。
- `--json --output` 使用同一个 `String`/字节切片；进程测试直接断言 stdout
  bytes 与目标文件 bytes 相等。
- JSON 由 `serde_json::to_string` 紧凑序列化后只追加一次 `\n`，因此当前
  合约有且只有一个稳定尾随换行。
- 成功目录断言只剩最终目标文件，不留下操作创建的临时文件。
- JSON 中未添加持久化 metadata、timestamp、random ID、hash、输出路径、
  storage path 或 model prose。

## Failure Behavior

- 现有目标被拒绝，命令非零退出，成功 stdout 为空，sentinel bytes 保持
  不变，目录内没有残留临时文件。
- 缺失父目录被拒绝；父目录和目标均不会被创建，成功 stdout 为空。
- CSV/分析失败发生在序列化和 storage 执行之前，不创建输出目标。
- persistence 失败通过 `CliError::User` 输出人类可读 stderr，不发出成功
  stdout。
- 临时文件写入、同步或 hard-link 发布失败时，executor 只尝试删除本次
  创建的临时文件；它不会删除预先存在的 stale temp 文件。
- 未发现 overwrite fallback、`--force`、check-then-write 或目标预删除路径。

## Storage Execution Status

- CLI 通过 `AtomicWriteRequest`、`WriteMode::CreateNew` 和
  `AtomicWritePlan::execute(&[u8])` 复用 storage 执行边界。
- CLI 只通过 `fs::read_to_string` 读取输入，没有调用 `std::fs::write`。
- 生产 CLI 和 storage executor 没有使用 `Path::exists()` 进行
  check-then-write。
- 临时路径是目标同目录的确定性 sibling：`<target>.atomic-write.tmp`。
- `OpenOptions::create_new(true)` 以 no-clobber 语义创建临时文件。
- 临时文件完成 `write_all` 和 `sync_all` 后，通过 `fs::hard_link` 以
  no-clobber 语义发布最终目标。
- 不支持 hard link 的文件系统返回安全失败并清理本次临时文件；没有覆盖
  fallback。
- `WriteMode::ReplaceExisting` 仍显式返回 unsupported error，且不产生文件
  系统变化。
- executor 只检查父目录是否已经是目录，不调用 `create_dir` 或
  `create_dir_all`。
- storage 只接收 opaque bytes，不了解 JSON 或 kinetics，也不序列化、检查
  或修改内容。

## Concurrency Status

`atomic_create_new` 集成回归测试符合预定的有界竞争设计：

- 恰好创建两个 writer thread。
- 使用 `Barrier::new(2)` 同步开始。
- 没有 stress loop、重试循环或 sleep。
- 断言恰好一个 writer 成功、恰好一个 writer 安全失败。
- 失败只接受 `TargetAlreadyExists` 或临时 create-new 的 `WriteFailed`。
- 最终目标必须等于某一个完整 payload，并且等于成功 writer 的 payload。
- 最终目录只包含目标文件，临时 sibling 不存在。
- 测试使用 `CARGO_TARGET_TMPDIR`，只删除精确目标文件和精确测试目录。
- 同一 integration target 还覆盖精确 opaque/empty bytes、existing target、
  missing parent、stale temp 和 deferred `ReplaceExisting` 行为。
- `cargo test -p deepseek-science-storage --test atomic_create_new`：6 passed。

## JSON Contract Status

- persisted JSON 是有效 UTF-8 JSON；CLI 进程测试使用 `serde_json` 解析成功。
- JSON stdout 和 persisted file 共享一次
  `format_kinetics_analysis_json_output` 结果，没有第二次独立序列化。
- 顶层固定包含 `schema_version`、`command`、`input`、`columns`、
  `counts`、`fits`、`comparison` 和 `review`。
- `schema_version` 为 `kinetics.analysis.v1`。
- `command` 为 `kinetics.analyze`。
- 所有 fit 浮点值在序列化前通过 `finite_json_float`；NaN 和 infinity 被
  拒绝。
- comparison 使用 `finite_r_squared_mvp_heuristic` 和明确的 caution 字段。
- 测试拒绝 `definitive`、`true model`、`proved`、`proof` 和
  `final reaction order` 等确定性科学结论措辞。

## README and Help Accuracy

README 和实际 `kinetics analyze --help` 均准确说明：

- `--output <path>` 显式保存确定性 JSON；
- 默认无写入，文本仍是默认 stdout；
- `--json` 控制成功 stdout 格式；
- `--json --output` 的 stdout 与文件 JSON byte-identical；
- 现有目标不会被覆盖；
- 父目录不会被创建；
- 错误写入 stderr。

README 和 help 未声称支持 overwrite、`--force`、ArtifactManifest
persistence、run persistence、project/workspace storage、hash identity、
model-generated explanations、真实 DeepSeek API 执行、UI 或 TypeScript。

## Test Status

- `cargo fmt --check`：passed。
- `cargo check -p deepseek-science-storage`：passed。
- `cargo test -p deepseek-science-storage --lib`：16 passed。
- `cargo test -p deepseek-science-storage --test atomic_create_new`：6 passed。
- `cargo check -p deepseek-science-cli`：passed。
- `cargo test -p deepseek-science-cli --lib`：19 passed。
- `cargo test -p deepseek-science-cli --test kinetics_analyze_smoke`：12 passed。
- `cargo check --workspace`：passed。
- `cargo test --workspace --lib`：255 passed。

CLI 输出测试覆盖：

- text stdout + JSON file；
- JSON stdout/file byte identity；
- existing target rejection 和 sentinel 保持；
- missing parent rejection；
- analysis failure creates no file。

CLI 和 storage integration tests 使用 tiny payload/fixture、
`CARGO_TARGET_TMPDIR` 和精确清理。storage 的每个文件系统测试通过 process
ID 加 bounded atomic counter 创建独立子目录，在清理前检查目录状态，然后
仅调用 `remove_file` 和 `remove_dir` 删除自己创建的路径。storage 中已无
`std::env::temp_dir()` 或 `remove_dir_all` 测试路径。

## Dependency Status

- `cargo tree --workspace` 未发现 Phase 3 新增外部依赖。
- CLI 复用已有内部 `deepseek-science-storage` 依赖边。
- JSON 继续使用已有 workspace `serde_json`。
- 未发现 `tempfile`、`clap`、`reqwest`、`tokio`、`sqlx`、
  `rusqlite` 或 UI framework。
- 未发现 Node、Bun、npm、TypeScript 或 JavaScript 项目路径。

## Crate Boundary Status

- storage 保持 domain-neutral，只验证路径、计划写入并执行 opaque-byte
  CreateNew publication。
- CLI 拥有 JSON 格式、输出参数、stdout 选择和文件写入编排。
- chemistry 拥有 kinetics 验证、拟合、比较、review 和分析结果。
- common 保持 chemistry-neutral 和 file-IO-free。
- artifacts/core 不依赖 chemistry；依赖方向仅由 chemistry 和 CLI 指向
  通用 crate。
- kinetics analyze 路径不执行 model、tool、workflow、ArtifactManifest
  persistence 或 run persistence。

## Disk Safety Status

生产行为满足预定边界：

- 默认 CLI 只读一个输入文件，不写文件。
- 显式 `--output` 最多创建一个小型 sibling temp 和一个最终目标。
- 不自动创建父目录，不启动后台 writer，不创建 log、cache、workspace、
  run record 或 artifact tree。
- 不扫描或清理目录。
- Cargo build/test output 保持在 `../.cache/deepseek-science-target`。
- 本次审计未运行 `cargo clean`、cleanup script、doc/release build、watch、
  coverage、profiling、benchmark、stress loop 或广泛清理。
- 未创建手工 smoke 文件；仓库内只创建本报告。
- PR #31 已将全部 storage 文件系统执行测试迁移到
  `env!("CARGO_TARGET_TMPDIR")` 下；每个测试使用唯一精确子目录，不扫描或
  清理其他路径，测试成功后不留下文件或目录。
- PR #31 未修改 `AtomicWritePlan::execute` 或其他 production storage 行为。

## v0.3 Readiness

- Functional readiness: **yes**。
- Storage CreateNew/no-overwrite readiness: **yes**。
- CLI `--output`、default no-write 和 JSON byte identity readiness: **yes**。
- Version readiness: **yes**。
- Test path isolation readiness: **yes**。
- Release/tag readiness: **yes**。

功能、生产持久化、并发、测试隔离、版本、依赖和文档检查均未发现 release
blocker。当前 `main` 可以在本报告审阅并提交、并重新确认分支/工作区/标签
状态后创建 annotated `v0.3.0` tag。

## Blocking Issues

None.

## Non-blocking Follow-ups

- 增加更强的跨平台 hard-link/no-clobber 文件系统覆盖。
- 设计 persisted-byte hash，并明确区分 semantic hash 与 exact-byte hash。
- 编写 ArtifactManifest mapping RFC。
- 独立设计 `ReplaceExisting` 和 symlink/regular-file 策略。
- 后续增加真实实验室 UTF-16、TSV 或仪器导出适配器。

这些 deferred features 不阻塞 v0.3。

## Recommended Next Step

审阅并提交本次刷新后的审计报告。随后在独立 release task 中重新确认
`main` 与 `origin/main` 对齐、工作区干净且本地/远端仍不存在 `v0.3.0`，
再创建并推送 annotated `v0.3.0` tag；不要在该 tag task 中创建 GitHub
Release。

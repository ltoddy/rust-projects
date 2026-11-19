# tokstat

统计目录下文本文件经 tokenizer 处理后的 token 量。类似 cloc 统计代码行数，只是指标换成了 token。

## 输出

```console
$ tokstat ~/github/ltoddy/aries
 TOKENS   SHARE  FILE
175,467   24.3%  scripts/uv.lock
117,447   16.3%  Cargo.lock
  6,922    1.0%  crates/aries-runtime/src/tools/grep/tests.rs
  5,078    0.7%  crates/aries-acp/src/v1/prompt/session_update.rs
  4,773    0.7%  crates/aries-runtime/src/tools/task.rs
… 还有 516 个文件
---------------------
720,810  100.0%  521 files, 5 skipped
```

结果按 token 数降序，路径相对传入的目录。总计行给出文件数与被跳过的文件数。

## 安装

```console
$ cargo install tokstat
```

从源码构建：

```console
$ git clone https://github.com/ltoddy/tokstat
$ cd tokstat
$ cargo install --path .
```

## 用法

```
Usage: tokstat [PATH]

Arguments:
  [PATH]  要统计的目录或文件，缺省为当前工作目录

Options:
      --top <N>  最多展示的文件数，0 表示全部 [default: 20]
  -h, --help     Print help
  -V, --version  Print version
```

## 统计范围

遍历使用 [ignore](https://docs.rs/ignore)，行为与 ripgrep 一致：

- 尊重 `.gitignore`、`.git/info/exclude` 与全局 gitignore
- 跳过隐藏文件与目录
- 不跟随符号链接

以下文件会被跳过并计入 `skipped`：

- 二进制文件（前 1024 字节含 NUL，或以 `%PDF`、`\x89PNG` 开头）
- 非 UTF-8 编码的文件（包括 UTF-16）

读不了的目录（权限不足、IO 错误）计入 `dir errors`，只在非零时展示。

## 分词器

内置 `openai/gpt-oss-120b` 的 `tokenizer.json`——即 `o200k_harmony`，o200k 词表加上 harmony 对话格式的特殊 token。编译期用 `include_bytes!` 内嵌，无运行时下载，release 二进制约 32 MB（其中 27 MB 是词表）。

因此：

- 结果对应 o200k 系词表（GPT-4o、GPT-4.1、o 系列、gpt-oss）。
- 对 Llama、Qwen、DeepSeek 等模型，词表不同，数值只能作为量级参考。
- 每个文件独立编码，跨文件边界无法合并 token，结果会比「把所有文件拼成一段」略高。

## 性能

目录遍历与编码分离：遍历线程只产出路径，经有界 channel 交给 N 个 worker 并行编码，N 为逻辑核数。上面 521 个文件的示例在 12 核机器上约 0.4 秒，其中 250 毫秒用于加载词表。

## 许可

MIT

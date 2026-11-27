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

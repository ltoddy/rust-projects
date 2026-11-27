# rcloc

统计目录下各语言的代码行、注释行与空行，类似 cloc。用 Rust 实现，多线程并行读取文件。

## 输出

```console
$ rcloc ~/github/ltoddy/aries
LANGUAGE         FILES     SIZE  BLANK  COMMENT    CODE   SHARE
Rust               464   1.9 MB  5,207      230  50,902   97.4%
Markdown            44  50.1 KB    279        0     677    1.3%
Toml                17  13.3 KB     32        0     484    0.9%
Python               1   5.6 KB     41       50     124    0.2%
Protocol Buffer      1   1.2 KB     12        0      47    0.1%
Shell                1   2.1 KB     12        1      36    0.1%
---------------------------------------------------------------
Total              528   2.0 MB  5,583      281  52,270  100.0%
528 files, 0 skipped
```

结果固定按 `CODE` 降序（同值按名称升序），默认只展示前 20 行，`--top 0` 展示全部。`SHARE` 是该行 `CODE` 占全部 `CODE` 的比例。总计行给出文件数与被跳过的文件数。

`--by-file` 按文件统计，路径相对传入的目录：

```console
$ rcloc --by-file --top 4 ~/github/ltoddy/aries/crates/aries-runtime/src
FILE                     SIZE  BLANK  COMMENT    CODE   SHARE
tools/grep/tests.rs   27.0 KB     82        0     645    6.0%
tools/task.rs         20.0 KB     82        2     545    5.1%
session/hook.rs       11.1 KB     33        0     279    2.6%
tools/read/tests.rs   10.6 KB     45        6     272    2.5%
… 还有 169 行
-------------------------------------------------------------
Total                400.2 KB  1,833       41  10,694  100.0%
173 files, 0 skipped
```

## 安装

```console
$ cargo install rcloc
```

从源码构建：

```console
$ git clone https://github.com/ltoddy/cloc-rs
$ cd cloc-rs
$ cargo install --path .
```

## 用法

```
Usage: rcloc [OPTIONS] [PATH]

Arguments:
  [PATH]  要统计的目录或文件，缺省为当前工作目录

Options:
      --top <N>  最多展示的行数，0 表示全部 [default: 20]
      --by-file  按文件统计，而不是按语言
  -h, --help     Print help
  -V, --version  Print version
```

## 统计范围

遍历使用 [ignore](https://docs.rs/ignore)，行为与 ripgrep 一致：

- 尊重 `.gitignore`、`.ignore`、`.git/info/exclude` 与全局 gitignore
- 跳过隐藏文件与目录
- 不跟随符号链接

以上行为都不可配置，与 tokstat、ripgrep 的默认行为一致。要统计被忽略或隐藏的目录/文件，直接把路径作为 `PATH` 传入即可——根路径本身不参与过滤，例如 `rcloc node_modules`、`rcloc .config`。

语言按扩展名识别，目前支持 117 种（见 `src/count.rs` 里的 `Languages::new`）。扩展名未知的文件不参与统计，也不计入 `skipped`。

`skipped` 只统计「本该能统计却失败了」的文件：

- 二进制文件（前 1024 字节含 NUL，或以 `%PDF`、`\x89PNG` 开头）
- 非 UTF-8 编码的文件
- 读取失败的文件

读不了的目录计入 `dir errors`，只在非零时展示。

## 注释识别

每种语言可以配置单行注释符与多行注释符（`language!(语言, 扩展名, 单行, 多行)`）。判定顺序是：空行 → 单行注释 → 多行注释 → 代码。

- 一行里既有代码又有注释（如 `let a = 1; // 说明`）算作代码。
- 多行注释内部的行算注释，其中的空行仍算空行。
- 单行注释符做前缀匹配，所以 `//` 也会命中 `///` 与 `///!`；Rust 里显式列了三种写法，顺序在前者优先。

## 性能

遍历与统计分离：2 个遍历线程只产出路径，经有界 channel（容量 256）交给 N 个 worker 并行统计，N 为逻辑核数。上面 528 个文件的示例约 0.04 秒。

## 与 cloc 的差异

- 只做行数统计，不提供 cloc 的 `--diff`（两份目录的差异比较）。
- 注释识别基于扩展名对应的注释符表，不做语法解析，`.h` 之类归属固定的语言。

## 许可

MIT

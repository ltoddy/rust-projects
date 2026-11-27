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

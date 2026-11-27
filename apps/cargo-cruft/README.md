# cargo-cruft

删掉 Cargo `target` 目录里再也不会被用到的那部分产物。`cargo clean` 只能整个目录清空，`cargo-cruft` 把范围收窄到四类确定不会再被复用的垃圾：

- **packages that left Cargo.lock** —— 构建单元对应的包已经不在 `Cargo.lock` 里，产物再也不会被链接。
- **units whose artifacts are gone** —— `out` 目录还在，但里面只剩 `.d` 依赖信息，没有任何真正的产物，说明上一次构建被打断。
- **leftovers no build owns** —— `build/<package>/<hash>` 下有目录却没有 `fingerprint`，cargo 从未记录过这个单元，不归任何一次构建所有。
- **incremental data** —— `<profile>/incremental` 下的增量缓存，删掉只会让下一次构建慢一点。

## 输出

```console
$ cargo cruft
target: /Users/me/github/myapp/target
size:   4.6 MiB

packages that left Cargo.lock:
  aarch64-apple-darwin/release:
    serde                                           1 unit  900.0 KiB
  debug:
    serde 1.0.219                                   1 unit    2.0 MiB

units whose artifacts are gone:
  debug:
    regex                                           1 unit  512.0 KiB

leftovers no build owns:
  debug:
    3c6f8e2b0d1a5f47                                1 unit  256.0 KiB

incremental data:
  debug:
    myapp                                           1 config  700.0 KiB
    regex                                           1 config  300.0 KiB

reclaimable: 4.6 MiB of 4.6 MiB
deleted 7 paths, freed 4.6 MiB
```

先按 profile 分组、再按对象名聚合，右侧是该条目占用的磁盘空间；每个 profile 下每类只展示空间最大的 12
条。报告之后立刻执行删除，所以这条命令执行完就已经清理过了。

条目名的来源各不相同：`serde 1.0.219` 的版本号是从 dep-info 文件里反推出来的（目录名只带包名和哈希）；`3c6f8e2b0d1a5f47`
是没有归主的残留，只能显示哈希目录名；`incremental` 下的目录名形如 `<crate>-<hash>`，按 crate 名聚合。

## 安装

```console
$ cargo install cargo-cruft
```

从源码构建：

```console
$ git clone https://github.com/ltoddy/rust-projects
$ cd rust-projects
$ cargo install --path apps/cargo-cruft
```

## 用法

```
Usage: cargo cruft
```

没有参数。在项目根目录下运行，读取当前目录的 `target/` 与 `Cargo.lock`：`target/` 不存在时直接退出，`Cargo.lock`
读不到时只警告一声（此时无法判断哪些包已经离开依赖图，其余三类照常报告）。

## 细节

- 体积统计用 `st_blocks`（真实占用磁盘空间）而不是文件长度，因此稀疏文件也算得准。
- 同一 inode 只计一次。cargo 会把最终产物硬链接到 `<profile>/` 根目录和 `<profile>/examples/`
  ，这些副本跟着对应单元一起删除，既不会重复计入体积，也不会留下悬空残留。
- 带 build script 的单元不参与 "artifacts are gone" 的判定，因为这类单元的 `out` 目录本来就只放中间文件。
- 扫描前对每个 profile 的 `.cargo-lock` 加排它锁：有 cargo 正在写这个目录时立刻失败，不会删掉活跃构建的产物。
- 退出码：`0` 表示全部删除成功或本来就无可清理；`1` 表示有路径删除失败，明细在 stderr。

## 范围

只处理 `<profile>/build` 与 `<profile>/incremental` 两类目录，`target/doc`、`target/package` 等不予触碰。识别 profile
看目录里是否有 `.cargo-lock`、`build`、`incremental`、`deps`、`.fingerprint` 中的任意一项，因此 `target/<triple>/<profile>`
这样的交叉编译产物会作为独立 profile 单独统计。

尚未覆盖的情况：`Cargo.toml` 的 feature 组合或 `[profile]` 改动留下的单元、`target/tmp` 之类 cargo 自己的临时目录。这些都需要
cargo 自己裁决，`cargo clean` 依然是唯一可靠的兜底。

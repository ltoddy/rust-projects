# tokstat

统计目录下的文本文件经 tokenizer 处理后的 token 量。类似 cloc 统计代码行数，只是把指标换成了 token。

## 分词器

内置 `openai/gpt-oss-120b` 的 `tokenizer.json`（即 `o200k_harmony`，o200k 词表加上 harmony 对话格式的特殊 token），编译时内嵌进二进制，无需额外下载。

## 状态

早期开发中。当前只输出每个文件的 token 数；按语言分组、总计、进度条等尚未实现。

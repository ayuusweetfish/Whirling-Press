find-single-io-error = 输入/输出错误：{$error}
find-single-none-found = 没有找到文件
find-single-multiple-found = 存在多个文件

process-find-yaml = 无法定位 YAML：{$error}
process-parse-yaml = 解析 YAML 遇到错误：{$error}
process-find-csv = 无法定位 CSV：{$error}
process-csv-name-utf8 = CSV 文件不是有效的 UTF-8
process-csv-fields-utf8 = CSV 列名不是有效的 UTF-8
process-csv-fields-duplicate = CSV 列名 `{$field}` 存在重复
process-io = 处理文件 `{$path}` 时错误：{$error}

process-expr-nonexistent-field =
  元素 {$index}，表达式位置 {$pos} 引用的列名 `{$name}` 不存在

process-render-typst-compilation =
  { $count ->
    [1] 编译：{$message}
   *[other] 编译：{$message}（共 {$count}）
  }

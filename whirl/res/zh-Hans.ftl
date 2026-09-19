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

open-content-folder = 打开文件夹
or-drag-and-drop = 或拖放到此处

menu-page-size = 页面尺寸
menu-save = 保存
menu-export-pdf = 导出 PDF…
menu-export-pdf-seq = 导出 PDF 序列…
menu-close = 关闭

status-num-elements = {$n} 个元素
status-error = 错误：{$error}
status-saved = 已保存 {$path}
status-exported = 已导出 {$path}
status-nonexistent-field = 列名 {"{"}{$name}{"}"} 不存在

status-button-theme = 切换颜色主题
status-button-language = 语言 / Language
status-button-menu = 菜单

elements = 元素列表
hide-attrs = 隐藏属性面板
show-attrs = 显示属性面板

attr-heading-image = 图像
attr-heading-text = 文本

attr-position = 位置
attr-anchor = 锚点
attr-scale = 缩放
attr-rotation = 旋转
attr-size-limit = 尺寸限制
attr-image-source = 图源
attr-text-text = 文本
attr-text-family = 字体
attr-text-size = 字号
attr-text-weight = 字重
attr-text-italic = 斜体
attr-text-tracking = 字间距

anchor-tl = 左上
anchor-tc = 正上
anchor-tr = 右上
anchor-ml = 正左
anchor-mc = 正中
anchor-mr = 正右
anchor-bl = 左下
anchor-bc = 正下
anchor-br = 右下

list-move-up = 上移
list-remove = 移除
list-add = 新增

find-single-io-error = I/O error: {$error}
find-single-none-found = no file found
find-single-multiple-found = multiple files found

process-find-yaml = Could not find YAML file: {$error}
process-parse-yaml = Could not parse YAML: {$error}
process-find-csv = Could not find CSV file: {$error}
process-csv-name-utf8 = CSV file name is not valid UTF-8
process-csv-fields-utf8 = CSV fields are not valid UTF-8
process-csv-fields-duplicate = CSV field `{$field}` duplicate
process-io = I/O error on `{$path}`: {$error}
process-expr-nonexistent-field =
  Element {$index}, expression at characters {$start}-{$end} references nonexistent field `{$name}`
process-render-typst-compilation =
  Element {$index}, compilation failed: {$message}

open-content-folder = Open content folder
or-drag-and-drop = or drag-and-drop

menu-page-size = Page size
menu-save = Save
menu-export-pdf = Export PDF…
menu-export-pdf-seq = Export PDF sequence…
menu-close = Close

status-num-elements = {$n ->
  [one] 1 element
 *[other] {$n} elements
}
status-error = Error: {$error}
status-saved = Saved {$path}
status-exported = Exported {$path}
status-nonexistent-field = Nonexistent field {"{"}{$name}{"}"}
status-compilation-error = Compilation failed: {$message}

status-button-theme = Switch colour theme
status-button-language = Language / 语言
status-button-menu = Menu

elements = Elements
hide-attrs = Hide attributes
show-attrs = Show attributes

attr-heading-image = Image
attr-heading-text = Text

attr-position = Position
attr-anchor = Anchor
attr-scale = Scale
attr-rotation = Rotation
attr-skew = Skew
attr-size-limit = Size limit
attr-image-source = Source
attr-text-text = Text
attr-text-family = Family
attr-text-size = Size
attr-text-weight = Weight
attr-text-italic = Italic
attr-text-tracking = Tracking

anchor-tl = Top Left
anchor-tc = Top Center
anchor-tr = Top Right
anchor-ml = Middle Left
anchor-mc = Middle Center
anchor-mr = Middle Right
anchor-bl = Bottom Left
anchor-bc = Bottom Center
anchor-br = Bottom Right

list-move-up = Move up
list-remove = Remove
list-add = Add

modal-close-confirmation-message = Edited layout is not saved. Really close?
modal-close-confirmation-close = Close without saving
modal-close-confirmation-cancel = Cancel

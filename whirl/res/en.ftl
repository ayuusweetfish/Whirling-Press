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
  Element {$index}, expression at byte {$pos} references nonexistent field `{$name}`

process-render-typst-compilation =
  { $count ->
    [one] Compilation: {$message}
   *[other] Compilation: {$message} (total {$count})
  }

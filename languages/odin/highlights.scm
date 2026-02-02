; Preprocs

[
  (calling_convention)
  (tag)
] @keyword.directive

; Includes

[
  "import"
  "package"
] @keyword.import

; Keywords

[
  "foreign"
  "using"
  "struct"
  "enum"
  "union"
  "defer"
  "cast"
  "transmute"
  "auto_cast"
  "map"
  "bit_set"
  "bit_field"
  "matrix"
] @keyword

[
  "proc"
] @keyword.function

[
  "return"
  "or_return"
  "or_else"
  "or_break"
  "or_continue"
] @keyword.return

[
  "distinct"
  "dynamic"
] @keyword.storage

; Conditionals

[
  "if"
  "else"
  "when"
  "switch"
  "case"
  "where"
  "break"
  (fallthrough_statement)
] @keyword.conditional

((ternary_expression
  [
    "?"
    ":"
    "if"
    "else"
    "when"
  ] @keyword.conditional.ternary)
  (#set! "priority" 105))

; Repeats

[
  "for"
  "do"
  "continue"
] @keyword.repeat

; Variables

(identifier) @variable

; Namespaces

(package_declaration (identifier) @module)

(import_declaration alias: (identifier) @module)

(foreign_block (identifier) @module)

(using_statement (identifier) @module)

; Parameters

(parameter (identifier) @variable.parameter ":" "="? (identifier)? @constant)

(default_parameter (identifier) @variable.parameter ":=")

(named_type (identifier) @variable.parameter)

(call_expression argument: (identifier) @variable.parameter "=")

; Functions

(procedure_declaration . (identifier) @function)

(procedure_declaration (identifier) @function (procedure (block)))

(procedure_declaration (identifier) @function (procedure (uninitialized)))

(overloaded_procedure_declaration (identifier) @function)

; Built-in functions
((call_expression function: (identifier) @function.builtin)
  (#any-of? @function.builtin
    "len" "cap" "append" "make" "delete" "new" "free"
    "size_of" "align_of" "offset_of" "type_of" "type_info_of" "typeid_of"
    "min" "max" "abs" "clamp"
    "raw_data" "swizzle"
    "clear" "copy" "reserve" "resize" "shrink"
    "unordered_remove" "ordered_remove" "delete_key"
    "panic" "unreachable"))

(call_expression function: (identifier) @function.call)

; Types

(type (identifier) @type)

((type (identifier) @type.builtin)
  (#any-of? @type.builtin
    "bool" "byte" "b8" "b16" "b32" "b64"
    "int" "i8" "i16" "i32" "i64" "i128"
    "uint" "u8" "u16" "u32" "u64" "u128" "uintptr"
    "i16le" "i32le" "i64le" "i128le" "u16le" "u32le" "u64le" "u128le"
    "i16be" "i32be" "i64be" "i128be" "u16be" "u32be" "u64be" "u128be"
    "f16" "f32" "f64" "f16le" "f32le" "f64le" "f16be" "f32be" "f64be"
    "complex32" "complex64" "complex128"
    "quaternion64" "quaternion128" "quaternion256"
    "rune" "string" "cstring" "rawptr" "typeid" "any"))

"..." @type.builtin

(struct_declaration (identifier) @type "::")

(enum_declaration (identifier) @type "::")

(union_declaration (identifier) @type "::")

(const_declaration (identifier) @type "::" [(array_type) (distinct_type) (bit_set_type) (pointer_type)])

(struct . (identifier) @type)

(field_type . (identifier) @module "." (identifier) @type)

(bit_set_type (identifier) @type ";")

(procedure_type (parameters (parameter (identifier) @type)))

(polymorphic_parameters (identifier) @type)

((identifier) @type
  (#match? @type "^[A-Z][a-zA-Z0-9]*$")
  (#not-has-parent? @type parameter procedure_declaration call_expression))

; Fields

(member_expression
  (call_expression
    function: (identifier) @function.call))

(member_expression "." (identifier) @variable.member)

(struct_type "{" (identifier) @variable.member)

(struct_field (identifier) @variable.member "="?)

(field (identifier) @variable.member)

; Constants

((identifier) @constant
  (#match? @constant "^_*[A-Z][A-Z0-9_]*$")
  (#not-has-parent? @constant type parameter))

(member_expression . "." (identifier) @constant)

(enum_declaration "{" (identifier) @constant)

; Macros

((call_expression function: (identifier) @function.macro)
  (#match? @function.macro "^_*[A-Z][A-Z0-9_]*$"))

; Attributes

(attribute (identifier) @attribute "="?)

; Labels

(label_statement (identifier) @label ":")

; Literals

(number) @number

(float) @number.float

(string) @string

(character) @character

(escape_sequence) @string.escape

(boolean) @boolean

[
  (uninitialized)
  (nil)
] @constant.builtin

((identifier) @variable.builtin
  (#eq? @variable.builtin "context"))

; Operators

[
  ":="
  "="
  "+"
  "-"
  "*"
  "/"
  "%"
  "%%"
  ; "%%="
  ">"
  ">="
  "<"
  "<="
  "=="
  "!="
  "~="
  "|"
  "~"
  "&"
  "&~"
  "<<"
  ">>"
  "||"
  "&&"
  "!"
  "^"
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "||="
  "&&="
  "&~="
  "..="
  "..<"
  "?"
] @operator

[
  "in"
  "not_in"
] @keyword.operator

; Punctuation

[ "{" "}" ] @punctuation.bracket

[ "(" ")" ] @punctuation.bracket

[ "[" "]" ] @punctuation.bracket

[
  "::"
  "->"
  "."
  ","
  ":"
  ";"
] @punctuation.delimiter


[
  "@"
  "$"
] @punctuation.special

; Comments

[
  (comment)
  (block_comment)
] @comment

; Errors

(ERROR) @error

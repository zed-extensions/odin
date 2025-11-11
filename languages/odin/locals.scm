; Scopes

[
  (block)
  (procedure_declaration)
  (for_statement)
  (if_statement)
  (when_statement)
  (switch_statement)
] @scope

; References

(identifier) @reference

; Definitions

; Namespaces and imports
(package_declaration (identifier) @definition.namespace)
(import_declaration alias: (identifier) @definition.namespace)

; Functions/procedures
(procedure_declaration (identifier) @definition.function)

; Type definitions
(struct_declaration (identifier) @definition.type "::")
(enum_declaration (identifier) @definition.enum "::")
(union_declaration (identifier) @definition.type "::")
(const_type_declaration (identifier) @definition.type ":")

; Variables and constants
(variable_declaration (identifier) @definition.var ":=")
(const_declaration (identifier) @definition.constant "::")

; Function parameters
(parameter (identifier) @definition.parameter ":"?)
(default_parameter (identifier) @definition.parameter ":=")

; Struct fields
(field (identifier) @definition.field ":")

; Enum values
; These are defined inside enum declarations
(enum_declaration (identifier) @definition.constant)

; For loop variables
; For loops implicitly declare one or two loop variables
; Examples: for i in 0..<10, for value in array, for val, idx in array
(for_statement (identifier) @definition.var)

; Labels for break/continue
(label_statement (identifier) @definition ":")

; Procedures
(procedure_declaration (identifier) @name (procedure "proc" @context)) @item
(overloaded_procedure_declaration (identifier) @name "proc" @context) @item

; Type declarations
(struct_declaration (identifier) @name "::" "struct" @context) @item
(enum_declaration (identifier) @name "::" "enum" @context) @item
(union_declaration (identifier) @name "::" "union" @context) @item

; Type aliases and constants
(const_type_declaration (identifier) @name ":" (type) @context) @item
(const_declaration (identifier) @name "::" [(array_type) (distinct_type) (bit_set_type) (pointer_type)] @context) @item
(const_declaration (identifier) @name "::" @context) @item

; Foreign blocks
(foreign_block (identifier) @name) @item

; Top-level variables
(variable_declaration (identifier) @name ":=" @context) @item

; Struct fields and enum values
(field ((identifier) @name ":" (type) @context) @item)
(enum_declaration "{" ((identifier) @name) @item)

(procedure_declaration (identifier) @name (procedure "proc" @context)) @item
(overloaded_procedure_declaration (identifier) @name "proc" @context) @item

(struct_declaration (identifier) @name "::" "struct" @context) @item
(enum_declaration (identifier) @name "::" "enum" @context) @item
(union_declaration (identifier) @name "::" "union" @context) @item
(const_declaration (identifier) @name "::" [(array_type) (distinct_type) (bit_set_type) (pointer_type)] @context) @item

(foreign_block (identifier) @name) @item

(field ((identifier) @name ":" (type) @context) @item)
(enum_declaration "{" ((identifier) @name) @item)

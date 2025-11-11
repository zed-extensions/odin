(block) @debug-scope
(procedure_declaration) @debug-scope
(struct_declaration) @debug-scope
(enum_declaration) @debug-scope
(union_declaration) @debug-scope
(for_statement) @debug-scope
(if_statement) @debug-scope
(when_statement) @debug-scope
(switch_statement) @debug-scope

(variable_declaration
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(const_declaration
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(const_type_declaration
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(parameter
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(default_parameter
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(field
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(for_statement
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(assignment_statement
  (identifier) @debug-variable
  (#not-eq? @debug-variable "_"))

(identifier) @debug-variable
  (#not-eq? @debug-variable "_")

; Procedures
(procedure_declaration
  (procedure
    (block
      "{"
      (_)* @function.inside
      "}"))) @function.around
(overloaded_procedure_declaration) @function.around

; Type declarations
[
  (struct_declaration)
  (enum_declaration)
  (union_declaration)
  (bit_field_declaration)
] @class.around

; Comments
(comment)+ @comment.around
(block_comment) @comment.around

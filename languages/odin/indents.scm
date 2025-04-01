(switch_case) @indent

; no `(_ "{" @start "}" @end) @indent` because the convention is to not indent
; the `case` statements in a switch
(block "{" @start "}" @end) @indent
(bit_field_declaration "{" @start "}" @end) @indent
(enum_declaration "{" @start "}" @end) @indent
(struct_declaration "{" @start "}" @end) @indent
(union_declaration "{" @start "}" @end) @indent
(overloaded_procedure_declaration "{" @start "}" @end) @indent
(struct "{" @start "}" @end) @indent
(map "{" @start "}" @end) @indent
(union_type "{" @start "}" @end) @indent
(matrix "{" @start "}" @end) @indent

(_ "(" @start ")" @end) @indent
(_ "[" @start "]" @end) @indent

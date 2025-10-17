; Main procedure
((procedure_declaration
  (identifier) @run @_name
  (procedure (block))
)
(#eq? @_name "main")
(#set! tag odin-main))

; Test procedures with @(test) attribute
((procedure_declaration
  (attributes
    (attribute "@" "(" (identifier) @_attr_name ")")
  )
  (identifier) @run
  (procedure
    (parameters
      (parameter)
    )
    (block)
  )
)
(#eq? @_attr_name "test")
(#set! tag odin-test))

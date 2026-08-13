((comment) @injection.content
  (#set! injection.language "comment"))

; glsl ------------------------------------------------------------------------
([
  ((comment) @_comment
    .
    (var_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (assignment_statement
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (const_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
]
  (#match? @_comment "(?i)^//\\s*glsl\\s*$")
  (#set! injection.language "glsl"))

; hlsl ------------------------------------------------------------------------
([
  ((comment) @_comment
    .
    (var_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (assignment_statement
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (const_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
]
  (#match? @_comment "(?i)^//\\s*hlsl\\s*$")
  (#set! injection.language "hlsl"))

; wgsl ------------------------------------------------------------------------
([
  ((comment) @_comment
    .
    (var_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (assignment_statement
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (const_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
]
  (#match? @_comment "(?i)^//\\s*wgsl\\s*$")
  (#set! injection.language "wgsl"))

; json ------------------------------------------------------------------------
([
  ((comment) @_comment
    .
    (var_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (assignment_statement
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (const_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
]
  (#match? @_comment "(?i)^//\\s*json\\s*$")
  (#set! injection.language "json"))

; sql -------------------------------------------------------------------------
([
  ((comment) @_comment
    .
    (var_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (assignment_statement
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (const_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
]
  (#match? @_comment "(?i)^//\\s*sql\\s*$")
  (#set! injection.language "sql"))

; html ------------------------------------------------------------------------
([
  ((comment) @_comment
    .
    (var_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (assignment_statement
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (const_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
]
  (#match? @_comment "(?i)^//\\s*html\\s*$")
  (#set! injection.language "html"))

; xml -------------------------------------------------------------------------
([
  ((comment) @_comment
    .
    (var_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (assignment_statement
      (string
        "`"
        (string_content) @injection.content
        "`")))
  ((comment) @_comment
    .
    (const_declaration
      (string
        "`"
        (string_content) @injection.content
        "`")))
]
  (#match? @_comment "(?i)^//\\s*xml\\s*$")
  (#set! injection.language "xml"))

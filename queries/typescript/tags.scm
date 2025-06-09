;; -----------------------------------------
;; From the original file (interfaces, etc.)
;; -----------------------------------------

(function_signature
  name: (identifier) @name) @definition.function

(method_signature
  name: (property_identifier) @name) @definition.method

(abstract_method_signature
  name: (property_identifier) @name) @definition.method

(module
  name: (identifier) @name) @definition.module

(interface_declaration
  name: (type_identifier) @name) @definition.interface


;; -----------------------------------------
;; NEW, more comprehensive queries
;; -----------------------------------------

; Standard function declarations: function foo() {}
(function_declaration
  name: (identifier) @name) @definition.function

; Standard class declarations: class Foo {}
(class_declaration
  name: (type_identifier) @name) @definition.class

(abstract_class_declaration
  name: (type_identifier) @name) @definition.class

; Arrow functions assigned to a const/let: const foo = () => {}
; (lexical_declaration
;   (variable_declarator
;     name: (identifier) @name
;     value: [(arrow_function) (function)])) @definition.function

; Exported functions and classes: export function foo() {}
(export_statement
  declaration: [
    (function_declaration
      name: (identifier) @name) @definition.function
    (class_declaration
      name: (type_identifier) @name) @definition.class
    (lexical_declaration
      (variable_declarator
        name: (identifier) @name
        value: (arrow_function))) @definition.function
  ]
)

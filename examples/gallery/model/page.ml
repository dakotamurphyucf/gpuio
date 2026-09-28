open Core

type t =
  | Presentation
  | Controls
  | Text_inputs
  | Numeric_inputs
[@@deriving equal, compare, sexp_of]

let all = [ Presentation; Controls; Text_inputs; Numeric_inputs ]

let title = function
  | Presentation -> "Presentation"
  | Controls -> "Selection & actions"
  | Text_inputs -> "Text editing"
  | Numeric_inputs -> "Numbers & codes"
;;

let description = function
  | Presentation -> "Expressive building blocks for thoughtful interfaces."
  | Controls -> "Native controls that respond to pointer, keyboard and assistive tools."
  | Text_inputs -> "Native editing, Unicode, selection and composition."
  | Numeric_inputs -> "Precise quantities, ranges, ratings and verification codes."
;;

let key = function
  | Presentation -> "presentation"
  | Controls -> "controls"
  | Text_inputs -> "text-inputs"
  | Numeric_inputs -> "numeric-inputs"
;;

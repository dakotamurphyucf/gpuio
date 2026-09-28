open Core

type t =
  | Presentation
  | Controls
  | Text_inputs
  | Numeric_inputs
  | Pickers
  | Overlays
  | Navigation
[@@deriving equal, compare, sexp_of]

let all =
  [ Presentation; Controls; Text_inputs; Numeric_inputs; Pickers; Overlays; Navigation ]
;;

let title = function
  | Presentation -> "Presentation"
  | Controls -> "Selection & actions"
  | Text_inputs -> "Text editing"
  | Numeric_inputs -> "Numbers & codes"
  | Pickers -> "Dates & colors"
  | Overlays -> "Overlays & help"
  | Navigation -> "Navigation & layout"
;;

let description = function
  | Presentation -> "Expressive building blocks for thoughtful interfaces."
  | Controls -> "Native controls that respond to pointer, keyboard and assistive tools."
  | Text_inputs -> "Native editing, Unicode, selection and composition."
  | Numeric_inputs -> "Precise quantities, ranges, ratings and verification codes."
  | Pickers -> "Explore a draft, then confirm the value that matters."
  | Overlays -> "Focused decisions and helpful context, without losing your place."
  | Navigation -> "Keep your place as the workspace grows around you."
;;

let key = function
  | Presentation -> "presentation"
  | Controls -> "controls"
  | Text_inputs -> "text-inputs"
  | Numeric_inputs -> "numeric-inputs"
  | Pickers -> "pickers"
  | Overlays -> "overlays"
  | Navigation -> "navigation"
;;

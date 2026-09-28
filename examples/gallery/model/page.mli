open Core

(** Stable section identity, independent of the displayed title or list position.
    Each page owns its preview state; switching pages unmounts native previews. *)
type t =
  | Presentation
  | Controls
  | Text_inputs
  | Numeric_inputs
  | Pickers
  | Overlays
  | Navigation
  | Feedback
  | Journeys
  | Collections
  | Documents
  | Canvas
  | Assets
  | Charts
  | Motion
  | Extensions
  | Responsive
  | Runtime
[@@deriving equal, compare, sexp_of]

val all : t list
val title : t -> string
val description : t -> string
val key : t -> string

open Core
module Field = Accessibility.Field

(** Application-owned validation and ordinary view composition. No controller,
    validation runtime, asynchronous work, or editor replacement is introduced. *)
module Layout : sig
  type t =
    | Vertical
    | Horizontal
  [@@deriving equal, sexp_of]
end

(** Display a label, the supplied native control, and optional help/error text.
    Apply semantic associations directly to [control]. Stable internal keys keep
    the control mounted when help/errors appear or disappear. Supported roots
    are those accepted by [View.with_accessibility] for a field.

    Defaults: vertical layout, 8px gap, muted help and red error text. Custom
    styles refine those defaults. The label is static text, not another Tab stop.
    [Horizontal] gives the label 160 logical pixels and lets content flex. *)
val field
  :  Field.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?layout:Layout.t
  -> ?label_style:Style.t
  -> ?help_style:Style.t
  -> ?error_style:Style.t
  -> control:'action View.t
  -> unit
  -> 'action View.t Or_error.t

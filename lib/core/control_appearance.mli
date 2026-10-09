open Core

module Label_position : sig
  type t =
    | Before
    | After
  [@@deriving equal, sexp_of]
end

(** Presentation for checkbox, switch and radio indicators, independent of the
    application's selected value. Geometry uses logical pixels. *)
type t [@@deriving equal, sexp_of]

(** Size is 8..128 (default 18), switch width is size..256 (default size * 5/3),
    and gap is 0..128 (default 8). All are finite. Label defaults to [After].

    Both styles accept Base/Checked/Indeterminate/Disabled; Disabled wins over
    checked/mixed state. Indicator accepts background, foreground, opacity,
    border widths/color and corner radii. Mark accepts foreground and opacity.
    At most 128 normalized declarations across both styles. Unsupported states
    and properties return an error. Root text styling remains independent. *)
val create
  :  ?size:float
  -> ?switch_width:float
  -> ?gap:float
  -> ?label_position:Label_position.t
  -> ?indicator_style:Style.t
  -> ?mark_style:Style.t
  -> unit
  -> t Or_error.t

val default : t
val size : t -> float
val switch_width : t -> float
val gap : t -> float
val label_position : t -> Label_position.t

module Expert : sig
  (** Resolve theme tokens before submission; unresolved colors are errors. *)
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Wire.Control_appearance.t Or_error.t
end

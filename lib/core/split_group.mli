open Core

(** Flat native panel groups. Rust owns measured sizes and gestures; OCaml owns
    panel identity, content, constraints and one-shot resize requests.
    Construct the native view with [View.split_group]. *)
module Id : sig
  type t [@@deriving equal, compare, sexp_of]

  (** Nonempty, case-sensitive UTF-8 without NUL; at most 256 bytes. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Axis = Split_pane.Axis

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** Paint is separate from the native hit target. Thickness defaults to 1
      (1–16 logical pixels); hit extent defaults to 8 (8–32), at least thickness.
      Styles refine the painted divider or custom grip, never its input geometry.
      Thickness sizes the default divider; custom grips use their own dimensions.
      Supported states: Base, Hovered, Focused, Pressed, Disabled. Supported
      properties are color, opacity, border, radius, shadow and text presentation;
      layout, visibility and input-policy overrides are rejected. Per-ID styles
      override shared styles within each state; state precedence is base, hover,
      focus, pressed, disabled. At most 64 unique overrides and 256 declarations.
      Absent IDs are ignored, allowing an appearance to survive panel filtering. *)
  val create
    :  ?thickness:float
    -> ?hit_extent:float
    -> ?handle_style:Style.t
    -> ?item_styles:(Id.t * Style.t) list
    -> unit
    -> t Or_error.t

  val default : t
end

module Panel : sig
  type t [@@deriving equal, sexp_of]

  (** Labels are nonblank, at most 1024 UTF-8 bytes. Default range: 80–16384
      logical pixels. An omitted initial size shares space left by explicit
      preferences. Bounds are finite, ordered and nonnegative; an explicit seed
      lies in the range. Hidden panes retain their identity/size. *)
  val create
    :  Id.t
    -> label:string
    -> ?initial_size:float
    -> ?minimum_size:float
    -> ?maximum_size:float
    -> ?visible:bool
    -> unit
    -> t Or_error.t

  val id : t -> Id.t
  val label : t -> string
  val is_visible : t -> bool
end

module Resize_request : sig
  type t [@@deriving equal, sexp_of]

  (** Positive monotonically increasing serial; finite target size 0–16384.
      The renderer will clamp to the target/sibling ranges. *)
  val create : Id.t -> size:float -> serial:int64 -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  val max_panels : int

  (** 0–64 unique panels. Keyboard step defaults to 16; reset generation to 0.
      A missing resize target is permitted and cancels that request. *)
  val create
    :  label:string
    -> ?axis:Axis.t
    -> ?keyboard_step:float
    -> ?reset_generation:int64
    -> ?resize:Resize_request.t
    -> Panel.t list
    -> t Or_error.t

  val panels : t -> Panel.t list
end

module Source : sig
  type t =
    | Pointer
    | Keyboard
    | Accessibility
    | Request of int64
  [@@deriving equal, sexp_of]
end

module Snapshot : sig
  type t [@@deriving equal, sexp_of]

  (** All panel IDs in current order, including hidden retained sizes. *)
  val sizes : t -> (Id.t * float) list

  val source : t -> Source.t
end

module Expert : sig
  val appearance_to_wire
    :  Appearance.t
    -> theme:Theme.t
    -> Gpuio_protocol.Wire.Split_group_appearance.t Or_error.t

  val to_wire : Config.t -> Gpuio_protocol.Split_group_wire.Config.t

  val snapshot_of_wire
    :  Config.t
    -> Gpuio_protocol.Split_group_wire.Snapshot.t
    -> Snapshot.t Or_error.t

  val generation : Config.t -> int64
end

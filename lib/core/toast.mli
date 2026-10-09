open Core

module Timeout : sig
  type t [@@deriving equal, sexp_of]

  val persistent : t

  (** Positive active time, at most 24 hours. Hidden or blocked notifications and
      stacks containing pointer hover or keyboard focus pause their timers. *)
  val after : Time_ns.Span.t -> t Or_error.t
end

module Politeness : sig
  type t =
    | Polite
    | Assertive
  [@@deriving equal, sexp_of]
end

module Dismissal : sig
  type t =
    | Timeout
    | Close_button
    | Escape
    | Overflow
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Defaults: five seconds of active time, polite announcement and "Dismiss
      notification" as the close-button label. Labels are nonblank UTF-8 without
      NUL; notification label <=4096 bytes, close label <=256 bytes. *)
  val create
    :  label:string
    -> ?timeout:Timeout.t
    -> ?politeness:Politeness.t
    -> ?close_label:string
    -> unit
    -> t Or_error.t
end

module Corner : sig
  type t =
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving equal, sexp_of]
end

module Placement : sig
  module Anchor : sig
    type t =
      | Top_left
      | Top_right
      | Bottom_left
      | Bottom_right
      | Top_center
      | Bottom_center
      | Left_center
      | Right_center
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  (** Window-edge insets default to 16 logical pixels, each finite in 0..16384.
      Center anchors align inside the inset rectangle. If opposing insets exceed
      the window extent they shrink proportionally, leaving zero usable extent. *)
  val create
    :  anchor:Anchor.t
    -> ?top:float
    -> ?right:float
    -> ?bottom:float
    -> ?left:float
    -> unit
    -> t Or_error.t
end

module Stack : sig
  module Motion : sig
    type t [@@deriving equal, sexp_of]

    (** Native reflow springs and finite slide/fade. Defaults: spring 400/40/1,
        epsilon 0.01 and two-second cap; 400ms entry, 200ms exit, 96px offset.
        Durations are in 0..60 seconds and round up to whole milliseconds.
        Offset is finite in 0..16384 logical pixels. No OCaml frame callbacks. *)
    val create
      :  ?spring:Animation.Spring.t
      -> ?enter:Time_ns.Span.t
      -> ?exit:Time_ns.Span.t
      -> ?offset:float
      -> unit
      -> t Or_error.t

    val default : t
  end

  module Layering : sig
    type t [@@deriving equal, sexp_of]

    (** Collapsed cards peek by 14px by default; expanded gap is 14px. Insets
        are finite in 0..16384; width_step is a fraction in 0..0.1 (default 0.05),
        visible layers are 1..8 (default 3). Hover or scope focus expands the
        retained cards. Only the front card is interactive while collapsed. *)
    val create
      :  ?peek:float
      -> ?gap:float
      -> ?width_step:float
      -> ?visible:int
      -> unit
      -> t Or_error.t

    val default : t
  end

  type t [@@deriving equal, sexp_of]

  (** Defaults: label "Notifications", bottom right, width 360 logical pixels,
      three visible notifications. Width must be finite/positive <=1,000,000;
      max_visible is 1..8. Older overflow is dismissed, never retained as an
      unbounded hidden queue. At most 32 keyed toast views may be submitted.
      [placement] supplies eight anchors and window insets; it cannot be combined
      with [corner]. Placement updates preserve native owners and timeout state.
      [layering] opts into overlapping cards. A named stack focus stop expands
      before older controls are traversed; Page Up/Down and Home/End scroll while
      that stop is focused. Hidden back cards pause expiry. Omitting [layering]
      retains the expanded scrolling column and its oldest-first order.
      [motion] opts into native reflow and entry/exit. Entry starts on first paint;
      active-time expiry starts after entry. Motion also supplies the named Group
      entry for native keyboard scrolling when layering is absent. Native dismissal
      retires input/focus
      immediately and publishes once after exit. Reduced motion, inactive/hidden
      owners or removing [motion] settle phases; removing the node discards pending
      dismissal. Ordinary config updates never replay entry or reopen a closed key.
      Omitting [motion] preserves immediate dismissal. *)
  val create
    :  ?label:string
    -> ?corner:Corner.t
    -> ?placement:Placement.t
    -> ?layering:Layering.t
    -> ?motion:Motion.t
    -> ?width:float
    -> ?max_visible:int
    -> unit
    -> t Or_error.t

  val default : t
end

module Expert : sig
  val motion : Stack.t -> Gpuio_protocol.Toast_motion_wire.t option
  val layering : Stack.t -> Gpuio_protocol.Toast_layering_wire.t option
  val placement : Stack.t -> Gpuio_protocol.Toast_placement_wire.t option
  val to_wire : Config.t -> Gpuio_protocol.Wire.Toast.t
  val stack_to_wire : Stack.t -> Gpuio_protocol.Wire.Toast_stack.t

  (** A terminal native dismissal remains valid after an ordinary configuration
      update. The reconciler separately checks the live node and handler. *)
  val dismissal : Gpuio_protocol.Wire.Toast_dismissal.t -> Dismissal.t
end

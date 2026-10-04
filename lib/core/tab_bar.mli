open Core

module Variant : sig
  type t =
    | Tab
    | Outline
    | Pill
    | Segmented
    | Underline
  [@@deriving equal, sexp_of]
end

module Motion : sig
  (** Native indicator springs for Pill/Segmented/Underline and a synchronized
      Pill selected-foreground fade. First usable layout adopts its target;
      interruptions retain painted position and velocity. Selection/focus/reveal
      stay independent. Hidden, removed and reduced-motion owners do not keep
      scheduling frames. Omit [View.tab_bar]'s [motion] for static presentation. *)
  type t [@@deriving equal, sexp_of]

  (** Default spring: stiffness 400, damping 40, mass 1, epsilon 0.01 logical
      pixels, maximum two seconds. [color_duration] defaults to 200 ms, permits
      zero and is at most 60 seconds; positive fractions round up to milliseconds.
      Tab/Outline keep their static selected visuals. *)
  val create
    :  ?spring:Animation.Spring.t
    -> ?color_duration:Time_ns.Span.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Appearance : sig
  (** Presentation of native tab targets; selection stays in [Choice.Config].
      Updating appearance retains native focus, active choice and label owners.
      Scrolling uses [Viewport]; interactive parts use [View.Tab_content].
      Native indicator animation is configured separately with [Motion]. *)
  type t [@@deriving equal, sexp_of]

  (** Defaults: Tab variant, 32-pixel height, 4-pixel inter-tab gap and 12-pixel
      horizontal padding. Height is 16..256, gap/padding 0..128 logical pixels.
      Styles support Base/Focused/Hovered/Pressed/Selected/Disabled and bounded
      box, color and text presentation. Visibility, positioning, scrolling and
      input policy belong to the root/config and are rejected here.
      Per-ID styles override shared styles within each state. Absent IDs are
      ignored, allowing one presentation to survive filtering/reordering.
      At most 128 unique ID overrides and 256 total style declarations. *)
  val create
    :  ?variant:Variant.t
    -> ?height:float
    -> ?gap:float
    -> ?padding:float
    -> ?tab_style:Style.t
    -> ?item_styles:(Choice.Id.t * Style.t) list
    -> unit
    -> t Or_error.t

  val default : t
  val variant : t -> Variant.t
end

module Reveal_request : sig
  (** A one-shot request within a mounted viewport. Positive serials increase
      for new requests; retained, reused or older serials never scroll again. *)
  type t [@@deriving equal, sexp_of]

  val create : Choice.Id.t -> serial:int64 -> t Or_error.t
end

module Viewport : sig
  type t [@@deriving equal, sexp_of]

  (** Opt into a horizontal, non-wrapping native scroll viewport with
      non-shrinking tab targets. Root sizing/colors still apply; direction,
      wrapping and overflow are owned by this viewport. Bound its width.
      Selection updates alone do not scroll. Native keyboard navigation and
      assistive focus reveal the active target. Child focus uses ordinary native
      scroll reveal; offsets stay in Rust.
      [reveal] moves by the least distance once measured, without changing focus
      or selection. Missing targets are consumed; removing a pending request
      cancels it. Zero-size/hidden viewports wait without scheduling idle frames.
      Removing the viewport retires its request history and offset; re-enabling
      creates a new viewport lifetime. Requests are not acknowledgements. *)
  val create : ?reveal:Reveal_request.t -> unit -> t

  val default : t
end

module Menu : sig
  (** An always-available all-tabs menu outside the scrolling viewport. Rows use
      full configured names, even for icon-only tabs, and current choice order,
      selection and disabled state. Selection uses the tab bar callback without
      implicitly revealing the tab. The trigger shows a caret; [label] names it
      for accessibility. Native focus, typeahead and popup scrolling stay in Rust. *)
  type t [@@deriving equal, sexp_of]

  (** Default label: "All tabs"; 1..1024 UTF-8 bytes without NUL. [style] styles
      the fixed trigger; [appearance] styles the bounded popup and its rows.
      [icons] supplies decorative SVGs beside full names. IDs are unique and
      bounded by [Choice.Collection.max_choices]; the frame rejects IDs absent
      from its current choices. Icon styles must be passive. Mounted icon readers
      retain native asset leases while the menu is closed or rows are offscreen;
      removing the icon/menu retires its reader. *)
  val create
    :  ?label:string
    -> ?style:Style.t
    -> ?appearance:Choice.Appearance.t
    -> ?icons:(Choice.Id.t * Icon.Decoration.t) list
    -> unit
    -> t Or_error.t

  val default : t
end

module Expert : sig
  val motion_to_wire : Motion.t -> Gpuio_protocol.Wire.Tab_motion.t
  val menu : Menu.t -> string * Style.t * Choice.Appearance.t
  val menu_icons : Menu.t -> (Choice.Id.t * Icon.Decoration.t) list
  val viewport_to_wire : Viewport.t -> Gpuio_protocol.Wire.Tab_viewport.t

  val to_wire
    :  Appearance.t
    -> theme:Theme.t
    -> Gpuio_protocol.Wire.Tab_appearance.t Or_error.t
end

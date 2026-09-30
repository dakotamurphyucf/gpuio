open Core

(** Native field composition for [Gpuio_bonsai.Settings.render_item]. The panel
    owns the visible item title; these helpers associate the native control with
    its semantic label/help/error and display help/error below it. Supply help in
    either item metadata or field metadata when duplicate visible text is unwanted.
    Values, resets, controllers and I/O remain application-owned. *)
val control
  :  Accessibility.Field.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?help_style:Style.t
  -> ?error_style:Style.t
  -> ?size:Presentation.Size.t
  -> layout:Settings.Layout.t
  -> control:'action View.t
  -> unit
  -> 'action View.t Or_error.t

(** Controlled Boolean fields. [on_toggle] is an intent, not a captured negated
    value: reduce it against latest application state. Disabled fields suppress
    native activation; their callbacks are still ordinary OCaml functions.
    Default size Medium; horizontal width 256px, vertical width 100%. Custom style
    refines that wrapper. Size affects spacing/font, not editor configuration. *)
val switch
  :  Accessibility.Field.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?size:Presentation.Size.t
  -> ?disabled:bool
  -> layout:Settings.Layout.t
  -> checked:bool
  -> on_toggle:(unit -> 'action)
  -> unit
  -> 'action View.t Or_error.t

val checkbox
  :  Accessibility.Field.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?size:Presentation.Size.t
  -> ?disabled:bool
  -> layout:Settings.Layout.t
  -> checked:bool
  -> on_toggle:(unit -> 'action)
  -> unit
  -> 'action View.t Or_error.t

module Choices : sig
  (** Typed values mapped to stable native IDs. IDs and application values must
      each be unique; duplicate display labels are allowed. The comparator defines
      value identity. Uses the normal Choice.Collection bounds and native popup
      virtualization; no string conversion or hash of an application value is
      inferred. Retain this collection across renders. *)
  type 'a t

  val create
    :  (module Comparator.S with type t = 'a and type comparator_witness = 'cmp)
    -> ('a * Choice.t) list
    -> 'a t Or_error.t

  (** Missing/foreign selected values are errors; [None] means no selection.
      A disabled selected option remains valid. Native activation requests the
      current typed value. Changing labels/order keeps surviving option identity.
      Popup dimensions, max visible rows, empty label and style are supplied by
      the ordinary [Choice.Appearance] API. *)
  val select
    :  'a t
    -> field:Accessibility.Field.t
    -> ?key:Key.t
    -> ?style:Style.t
    -> ?size:Presentation.Size.t
    -> ?disabled:bool
    -> ?appearance:Choice.Appearance.t
    -> layout:Settings.Layout.t
    -> selected:'a option
    -> on_select:('a -> 'action)
    -> unit
    -> 'action View.t Or_error.t
end

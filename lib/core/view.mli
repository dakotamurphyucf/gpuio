open Core

(** Pure, immutable UI descriptions. Actions need not be Bonsai effects: tests and
    other runtimes can use ordinary variants. Callbacks run only on the OCaml UI
    domain, after generation validation, using the latest accepted closure. *)
type 'action t

val text : ?key:Key.t -> ?style:Style.t -> string -> 'action t

(** An explicit accessible name must be nonempty and at most 1024 bytes;
    invalid names raise, as with literal styles built with [Style.create_exn]. *)
val button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> on_click:(unit -> 'action)
  -> string
  -> 'action t

(** Controlled application values. Activation emits an intent, never a Boolean
    computed from the last render. Apply it to the current model (for example
    [Bonsai.Cont.toggle] or a state machine using [Check_state.activate]). Disabled
    controls are inert, excluded from Tab traversal and invalidate their handler. *)
val checkbox
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> state:Check_state.t
  -> on_toggle:(unit -> 'action)
  -> string
  -> 'action t

val switch
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> checked:bool
  -> on_toggle:(unit -> 'action)
  -> string
  -> 'action t

val row : ?key:Key.t -> ?style:Style.t -> 'action t list -> 'action t
val column : ?key:Key.t -> ?style:Style.t -> 'action t list -> 'action t

val grid
  :  ?key:Key.t
  -> ?style:Style.t
  -> columns:int
  -> 'action t list
  -> 'action t Or_error.t

(** One controller identifies one placement across a window tree. Initial text is
    read only on native creation; use explicit editor commands for later edits. *)
val text_input
  :  ?style:Style.t
  -> ?initial_text:string
  -> controller:Key.t
  -> config:Text_input.Config.t
  -> on_event:(Text_input.Event.t -> 'action)
  -> unit
  -> 'action t Or_error.t

(** One native Tab stop. Arrow/Home/End keys navigate enabled options; native
    activation requests a stable option ID. OCaml owns the selected value. *)
val radio_group
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Choice.Config.t
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t

(** A native select with a bounded, scrollable option popup. Focus remains on
    the trigger. Arrows move the open popup highlight without changing the
    application value; Enter requests the highlighted ID, Escape cancels, and
    Tab closes before normal traversal. The label is shown when no value is
    selected. Printable text searches enabled labels without committing a value;
    repeated characters cycle, and the bounded prefix expires between inputs.
    Disabling or removing the control disposes its open popup. *)
val select
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Choice.Appearance.t
  -> config:Choice.Config.t
  -> on_select:(Choice.Id.t -> 'action)
  -> unit
  -> 'action t

(** One native editor placement with choice selection intents. Initial text is
    used only on mount; choosing never implicitly replaces the native query. *)
val combobox
  :  ?style:Style.t
  -> ?appearance:Choice.Appearance.t
  -> ?initial_text:string
  -> controller:Key.t
  -> config:Combobox.Config.t
  -> on_event:(Combobox.Event.t -> 'action)
  -> unit
  -> 'action t Or_error.t

module Expert : sig
  module Kind : sig
    type t =
      | Container
      | Text
      | Button
      | Input
      | Textarea
      | Checkbox
      | Switch
      | Radio_group
      | Select
      | Combobox
    [@@deriving equal, sexp_of]
  end

  module Control : sig
    type t =
      | Button of { disabled : bool }
      | Checkbox of
          { state : Check_state.t
          ; disabled : bool
          }
      | Switch of
          { checked : bool
          ; disabled : bool
          }
    [@@deriving equal, sexp_of]

    val to_wire : t -> Gpuio_protocol.Wire.Control.t
  end

  type 'action combobox =
    { controller : Key.t
    ; config : Combobox.Config.t
    ; appearance : Choice.Appearance.t
    ; on_event : Combobox.Event.t -> 'action
    }

  type 'action choice =
    { config : Choice.Config.t
    ; appearance : Choice.Appearance.t option
    ; on_select : Choice.Id.t -> 'action
    }

  type 'action editor =
    { controller : Key.t
    ; config : Text_input.Config.t
    ; on_event : Text_input.Event.t -> 'action
    }

  type 'action description =
    { key : Key.t option
    ; kind : Kind.t
    ; text : string
    ; style : Style.t
    ; on_click : (unit -> 'action) option
    ; editor : 'action editor option
    ; control : Control.t option
    ; choice : 'action choice option
    ; combobox : 'action combobox option
    ; children : 'action t list
    }

  val describe : 'action t -> 'action description
end

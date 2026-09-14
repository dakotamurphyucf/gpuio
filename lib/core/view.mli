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

(** Registry definitions are inherited by descendants; the nearest definition of
    an ID wins. A command button uses the registry's label/enabled state. Missing
    command references are rejected before a view update is submitted. *)
val command_scope
  :  ?key:Key.t
  -> ?style:Style.t
  -> commands:'action Command.Registry.t
  -> 'action t list
  -> 'action t

val command_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> command:Command.Id.t
  -> unit
  -> 'action t

(** Mounting opens a modal, native-owned search session. Search/navigation do not
    roundtrip through OCaml. Escape, permitted outside clicks, or selecting a
    command close it natively and restore the prior eligible focus. [on_dismiss]
    should remove the view. A closed session stays closed until unmounted and
    mounted again, or replaced with a new key; metadata updates do not reopen it.
    The query is independent of document editors and never becomes the target of
    registry native-edit commands. Commands resolve at the palette's tree location. *)
val command_palette
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Command_palette.Appearance.t
  -> config:Command_palette.Config.t
  -> on_dismiss:(Command_palette.Dismissal.t -> 'action)
  -> unit
  -> 'action t

(** Native-managed menu navigation resolves the same command registry as buttons
    and shortcuts. The context menu wraps one arbitrary child and opens on right
    click or Shift-F10. Escape restores prior focus. [menu_bar] defaults to the
    native menu bar on macOS and an in-window bar on Linux; [platform=false]
    renders an in-window bar on either platform. At most one platform bar may be
    mounted per window. Menus are replaced with the active window's definitions. *)
val menu_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> menu:Menu.t
  -> unit
  -> 'action t

val context_menu
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> menu:Menu.t
  -> 'action t
  -> 'action t

val menu_bar
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> ?platform:bool
  -> Menu.t list
  -> 'action t Core.Or_error.t

(** Native focus policy for the supplied subtree. Scope lifetime follows keyed
    mount/unmount; changing descendants preserves the scope's restoration target. *)
val focus_scope
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Focus_scope.t
  -> 'action t list
  -> 'action t

(** [None] closes and unmounts modal content. A dialog traps focus until closed.
    [style] applies to the panel, and content can contain any ordinary views. *)
val dialog
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Overlay.Config.t
  -> on_dismiss:(Overlay.Dismissal.t -> 'action)
  -> 'action t option
  -> 'action t

(** The anchor remains mounted when closed. Content is positioned against its
    current frame's bounds, enters focus without trapping, and restores on close. *)
val popover
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Overlay.Config.t
  -> on_dismiss:(Overlay.Dismissal.t -> 'action)
  -> anchor:'action t
  -> 'action t option
  -> 'action t

(** Content remains retained while native open state hides it. This preserves
    Bonsai models and editor buffers; unmount the tooltip to dispose its content.
    [style] customizes the tooltip panel. The anchor retains its own styles. *)
val tooltip
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Tooltip.Config.t
  -> ?on_open_change:(bool -> 'action)
  -> anchor:'action t
  -> content:'action t
  -> unit
  -> 'action t

val row : ?key:Key.t -> ?style:Style.t -> 'action t list -> 'action t
val column : ?key:Key.t -> ?style:Style.t -> 'action t list -> 'action t

val grid
  :  ?key:Key.t
  -> ?style:Style.t
  -> columns:int
  -> 'action t list
  -> 'action t Core.Or_error.t

(** One controller identifies one placement across a window tree. Initial text is
    read only on native creation; use explicit editor commands for later edits. *)
val text_input
  :  ?style:Style.t
  -> ?initial_text:string
  -> controller:Key.t
  -> config:Text_input.Config.t
  -> on_event:(Text_input.Event.t -> 'action)
  -> unit
  -> 'action t Core.Or_error.t

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
  -> 'action t Core.Or_error.t

(** Encoded assets registered with [Gpuio_eio.Asset]. Layout comes from [style];
    [config] declares fit and accessibility. State changes are asynchronous and
    refer to the currently mounted source. Retired registrations keep existing
    mounts usable but cannot create new bindings. *)
val image
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_change:(Image.State.t -> 'action)
  -> Image.Config.t
  -> 'action t

(** SVG alpha mask tinted by the native foreground, with ordinary logical
    layout styles. Source bytes are registered once; size/tint updates stay native. *)
val icon
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_change:(Image.State.t -> 'action)
  -> Icon.Config.t
  -> 'action t

(** A noninteractive native progress bar. Root Background styles the track and
    Foreground styles the indicator. Indeterminate motion stays on the native side;
    updates retain node identity. The accessible value is a percentage or absent
    when indeterminate. Width/height use ordinary logical-pixel styles. *)
val progress
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Progress.Config.t
  -> unit
  -> 'action t

(** A keyed notification session, usable only as a toast-stack item. *)
type 'action toast

(** Native drag source/drop target regions with shared style and children.
    Callbacks observe native decisions; provide keyboard command alternatives. *)
val drag_source
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Drag_and_drop.Source.t
  -> on_event:(Drag_and_drop.Source_event.t -> 'action)
  -> 'action t list
  -> 'action t

val drop_target
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Drag_and_drop.Target.t
  -> on_event:(Drag_and_drop.Target_event.t -> 'action)
  -> 'action t list
  -> 'action t

(** Native captured mouse gestures with ordinary content. Root styles do not
    replace the captured node; hiding/unmounting releases native capture. *)
val pointer_area
  :  ?key:Key.t
  -> ?style:Style.t
  -> config:Pointer.Config.t
  -> on_event:(Pointer.Event.t -> 'action)
  -> 'action t list
  -> 'action t

(** Native dismissal closes the session once; the callback should remove it.
    Content changes preserve elapsed time; changing timeout resets its interval.
    Hidden items pause. A closed item only reopens with a new key or remount. *)
val toast
  :  key:Key.t
  -> ?style:Style.t
  -> config:Toast.Config.t
  -> on_dismiss:(Toast.Dismissal.t -> 'action)
  -> 'action t list
  -> 'action toast

(** Newest items occupy the visible stack; older excess receives Overflow.
    Rejects duplicate keys or more than 32 submitted items. *)
val toast_stack
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?config:Toast.Stack.t
  -> 'action toast list
  -> 'action t Core.Or_error.t

module Expert : sig
  type 'action image =
    { config : Image.Config.t
    ; on_change : (Image.State.t -> 'action) option
    }

  type 'action drag_source =
    { config : Drag_and_drop.Source.t
    ; on_event : Drag_and_drop.Source_event.t -> 'action
    }

  type 'action drop_target =
    { config : Drag_and_drop.Target.t
    ; on_event : Drag_and_drop.Target_event.t -> 'action
    }

  type 'action pointer =
    { config : Pointer.Config.t
    ; on_event : Pointer.Event.t -> 'action
    }

  type 'action notification =
    { config : Toast.Config.t
    ; on_dismiss : Toast.Dismissal.t -> 'action
    }

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
      | Focus_scope
      | Tooltip
      | Command_scope
      | Command_button
      | Menu
      | Command_palette
      | Progress
      | Toast
      | Toast_stack
      | Pointer_area
      | Drag_source
      | Drop_target
      | Image
      | Icon
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

  type 'action overlay =
    { kind : Gpuio_protocol.Wire.Overlay_kind.t
    ; config : Overlay.Config.t
    ; on_dismiss : Overlay.Dismissal.t -> 'action
    }

  type 'action tooltip =
    { config : Tooltip.Config.t
    ; on_open_change : (bool -> 'action) option
    }

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

  type 'action palette =
    { config : Command_palette.Config.t
    ; appearance : Command_palette.Appearance.t
    ; on_dismiss : Command_palette.Dismissal.t -> 'action
    }

  type menu =
    { presentation : Menu.Expert.presentation
    ; menus : Menu.t list
    ; appearance : Menu.Appearance.t
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
    ; overlay : 'action overlay option
    ; tooltip : 'action tooltip option
    ; commands : 'action Command.Registry.t option
    ; command_ref : Command.Id.t option
    ; drag_source : 'action drag_source option
    ; drop_target : 'action drop_target option
    ; pointer : 'action pointer option
    ; notification : 'action notification option
    ; toast_stack : Toast.Stack.t option
    ; progress : Progress.Config.t option
    ; image : 'action image option
    ; palette : 'action palette option
    ; menu : menu option
    ; focus_scope : Focus_scope.t option
    ; children : 'action t list
    }

  val describe : 'action t -> 'action description
end

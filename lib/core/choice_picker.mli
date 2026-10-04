open Core

(** Validated values for the trigger-and-popup picker. [View.choice_picker]
    builds its native description; the Eio adapter supplies a Bonsai query
    controller. Existing Select and editable Combobox keep separate contracts. *)
module Group : sig
  module Id : sig
    type t [@@deriving equal, compare, sexp_of]

    (** 1..256 UTF-8 bytes without NUL, distinct from a choice identity. *)
    val of_string : string -> t Or_error.t

    val to_string : t -> string
  end

  type t [@@deriving equal, sexp_of]

  (** A nonselectable section with a 1..1024-byte accessible label. Empty groups
      are valid. Item IDs must also be unique across the entire collection. *)
  val create : id:Id.t -> label:string -> Choice.Collection.t -> t Or_error.t

  val id : t -> Id.t
  val label : t -> string
  val items : t -> Choice.Collection.t
end

module Collection : sig
  type t [@@deriving equal, sexp_of]

  val flat : Choice.Collection.t -> t

  (** At most 256 groups and 4096 total items. Group IDs and item IDs are unique
      in their respective namespaces. All group/item IDs and labels together
      require at most 262144 UTF-8 bytes. Preserve supplied order, including empty
      groups; native search may hide empty matches without deleting the catalog. *)
  val grouped : Group.t list -> t Or_error.t

  val groups : t -> Group.t list option
  val items : t -> Choice.t list
  val find : t -> Choice.Id.t -> Choice.t option
end

module Mode : sig
  type t =
    | Single
    | Multiple
  [@@deriving equal, sexp_of]
end

module Selection : sig
  type t [@@deriving equal, sexp_of]

  val single : Choice.Id.t option -> t

  (** Preserve supplied selection order; reject duplicates and more than 4096
      IDs. Config creation validates membership in the complete catalog. *)
  val multiple : Choice.Id.t list -> t Or_error.t

  val mode : t -> Mode.t
  val ids : t -> Choice.Id.t list
  val mem : t -> Choice.Id.t -> bool
end

module Search : sig
  type t =
    | None
    | Substring
    | Application
  [@@deriving equal, sexp_of]
end

module Open_state : sig
  type t =
    | Managed of { initially_open : bool }
    | Controlled of bool
  [@@deriving equal, sexp_of]
end

module Request : sig
  (** User intents, not replacement snapshots or acknowledgements. Apply them
      sequentially to current state. Select applies only to Single, Toggle only
      to Multiple; Clear applies to either mode. Transport lifetime fencing is
      separate from this pure reducer. *)
  type t =
    | Select of Choice.Id.t
    | Toggle of Choice.Id.t
    | Clear
  [@@deriving equal, sexp_of]
end

module Open_reason : sig
  type t =
    | Trigger
    | Keyboard
    | Escape
    | Outside_pointer
    | Focus_left
    | Selection
  [@@deriving equal, sexp_of]
end

module Visibility_reason : sig
  type t =
    | Interaction of Open_reason.t
    | Application
    | Unavailable
  [@@deriving equal, sexp_of]
end

module Visibility : sig
  type t =
    | Snapshot of bool
    | Changed of bool * Visibility_reason.t
  [@@deriving equal, sexp_of]
end

module Selection_request : sig
  type t [@@deriving equal, sexp_of]

  val request : t -> Request.t

  (** Exact native query at activation, present when search is enabled. Retains
      the search editor's lease/revision for conditional replacement. Never use a
      controller's last observed snapshot as a substitute. No active composition. *)
  val query : t -> Text_input.Snapshot.t option
end

module Event : sig
  (** A single ordered stream. Requests express intent; Visibility reports actual
      popup state. Controlled requests do not change visibility until accepted by
      a configuration update. Subscription begins with a Snapshot. Single-choice
      activation requests selection before closing; multiple-choice activation
      requests a toggle and keeps the popup open. Query changes retain the native
      editor lease and are delivered in the same ordered application callback. *)
  type t =
    | Selection_requested of Selection_request.t
    | Open_requested of bool * Open_reason.t
    | Visibility of Visibility.t
    | Query_changed of Text_input.Snapshot.t
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Selection must belong to the complete catalog, including disabled items.
      Applications using external search must preserve selected catalog entries.
      Label: 1..1024 UTF-8 bytes; placeholders: 0..1024 bytes, without NUL.
      Defaults: enabled, no search, not clearable, closed native-managed popup,
      empty placeholders. This constructs data and does not open a native window. *)
  val create
    :  label:string
    -> options:Collection.t
    -> selected:Selection.t
    -> ?disabled:bool
    -> ?search:Search.t
    -> ?clearable:bool
    -> ?open_state:Open_state.t
    -> ?placeholder:string
    -> ?search_placeholder:string
    -> unit
    -> t Or_error.t

  val label : t -> string
  val options : t -> Collection.t
  val selected : t -> Selection.t
  val is_disabled : t -> bool
  val search : t -> Search.t
  val is_clearable : t -> bool
  val open_state : t -> Open_state.t
  val placeholder : t -> string
  val search_placeholder : t -> string

  (** Reduce one request against this current configuration. Disabled controls,
      removed/disabled items and wrong-mode requests leave selection unchanged.
      Toggle appends a new ID or removes its existing occurrence. An enabled,
      clearable control clears the entire selection, including disabled items.
      Selection mode is preserved. No callback or I/O is performed. *)
  val apply_request : t -> Request.t -> Selection.t
end

module Query : sig
  type t [@@deriving equal, sexp_of]

  (** An editor placement key and mount-only single-line seed. Closing the popup
      retains this editor; removing the query placement retires its lease. *)
  val create : controller:Key.t -> ?initial_text:string -> unit -> t Or_error.t

  val controller : t -> Key.t
  val initial_text : t -> string
end

module Checkmark : sig
  type t =
    | Native
    | Custom
  [@@deriving equal, sexp_of]
end

module Option_content : sig
  type 'a t

  (** Native appends the ordinary selected indicator. Custom lets the supplied
      passive row draw its own indicator, using application-owned selection. *)
  val create : ?checkmark:Checkmark.t -> 'a -> 'a t

  val content : 'a t -> 'a
  val checkmark : 'a t -> Checkmark.t
end

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  val default : t

  (** Geometry is in logical pixels; popup bounds are additionally clamped by the
      native window. Width/max-height: positive finite <=1,000,000; estimate:
      1..1,000,000; overscan: 0..4096. Actual row heights are measured.
      Parts use Choice.Appearance's paint/text vocabulary. Header/empty accept
      Base only; popup Base/Hovered; option also Focused/Pressed/Selected/Disabled.
      The combined limit is 128 declarations. Root and footer layout stays on
      their Views. Defaults: width 320, height 320, estimate 32, overscan 64. *)
  val create
    :  ?popup_width:float
    -> ?max_height:float
    -> ?estimated_row_height:float
    -> ?overscan:float
    -> ?empty_label:string
    -> ?popup_style:Style.t
    -> ?option_style:Style.t
    -> ?header_style:Style.t
    -> ?empty_style:Style.t
    -> unit
    -> t Or_error.t
end

module Description : sig
  (** A checked content description. Content is deliberately generic so this data
      module does not depend on View. [View.choice_picker] additionally validates
      passive content and aggregate child budgets.
      Only the footer permits interactive descendants. Native options own input
      and accessibility; catalog labels remain their accessible names. *)
  type 'a t

  (** Validate unique, present IDs for optional group/option overrides. Query is
      required exactly when search is enabled. Missing overrides use native text.
      Custom trigger/empty/header/option content is passive. Changing overrides
      does not redefine choice identity. This allocates no native control. *)
  val create
    :  config:Config.t
    -> ?appearance:Appearance.t
    -> ?query:Query.t
    -> ?trigger:'a
    -> ?empty:'a
    -> ?footer:'a
    -> ?groups:(Group.Id.t * 'a) list
    -> ?options:(Choice.Id.t * 'a Option_content.t) list
    -> unit
    -> 'a t Or_error.t

  val config : 'a t -> Config.t
  val appearance : 'a t -> Appearance.t
  val query : 'a t -> Query.t option
  val trigger : 'a t -> 'a option
  val empty : 'a t -> 'a option
  val footer : 'a t -> 'a option
  val groups : 'a t -> (Group.Id.t * 'a) list
  val options : 'a t -> (Choice.Id.t * 'a Option_content.t) list
end

module Expert : sig
  (** Current-model selection/mode/disabled policy, after identity validation. *)
  val accepts_event : Config.t -> Event.t -> bool

  val to_wire : Config.t -> Gpuio_protocol.Choice_picker_wire.Config.t
  val of_wire : Gpuio_protocol.Choice_picker_wire.Config.t -> Config.t Or_error.t

  val description_to_wire
    :  'a Description.t
    -> theme:Theme.t
    -> Gpuio_protocol.Wire.Choice_picker_presentation.t Or_error.t

  (** Validate payload and expected search-editor identity. [query_node] is the
      current child editor, or None when search is disabled. The window comes
      from the routed envelope. The receiver must still check current picker and
      handler generations. Query observations may compose; selection may not. *)
  val event_of_wire
    :  Gpuio_protocol.Choice_picker_wire.Event.t
    -> window:Gpuio_protocol.Window_id.t
    -> query_node:Gpuio_protocol.Node_id.t option
    -> Event.t Or_error.t
end

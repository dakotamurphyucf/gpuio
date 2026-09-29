open Core

(** Stateless compositions. These helpers own no editors, tasks, registrations or
    controllers. All content slots accept ordinary views, including Bonsai effects.
    Application state and asynchronous work remain with the caller. *)
module Size : sig
  type t =
    | Small
    | Medium
    | Large
  [@@deriving equal, sexp_of]
end

module Tone : sig
  type t =
    | Neutral
    | Accent
    | Success
    | Warning
    | Danger
  [@@deriving equal, sexp_of]
end

module Variant : sig
  type t =
    | Soft
    | Outline
    | Solid
  [@@deriving equal, sexp_of]
end

module Axis : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Appearance : sig
  type t

  (** Concrete defaults work with existing custom themes without new tokens.
      Custom colors may use application tokens, resolved by the normal theme. *)
  val light : t

  val dark : t

  val create
    :  surface:Color.t
    -> raised:Color.t
    -> foreground:Color.t
    -> muted:Color.t
    -> border:Color.t
    -> on_solid:Color.t
    -> accent:Color.t
    -> success:Color.t
    -> warning:Color.t
    -> danger:Color.t
    -> t
end

(** Styles refine component defaults. Text wraps unless the caller requests
    truncation; empty/localized labels do not allocate hidden placeholder text. *)
val label : ?key:Key.t -> ?style:Style.t -> string -> 'action View.t

val badge
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?size:Size.t
  -> ?tone:Tone.t
  -> ?variant:Variant.t
  -> ?leading:'action View.t
  -> string
  -> 'action View.t

(** The optional trailing slot can hold a separately labelled remove button.
    The tag itself is not an action or an additional focus stop. *)
val tag
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?size:Size.t
  -> ?tone:Tone.t
  -> ?variant:Variant.t
  -> ?leading:'action View.t
  -> ?trailing:'action View.t
  -> string
  -> 'action View.t

(** A colored dot accompanied by readable text; color is never the only signal. *)
val marker
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> string
  -> 'action View.t

(** Uses native button keyboard/AX activation with Link semantics. Activation
    delivers the supplied action; opening a URL is an explicit application job. *)
val link
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?disabled:bool
  -> on_click:(unit -> 'action)
  -> string
  -> 'action View.t

val separator
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?axis:Axis.t
  -> unit
  -> 'action View.t

(** Stable header/body/footer wrappers preserve body identity as slots change. *)
val group_box
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?header:'action View.t
  -> ?footer:'action View.t
  -> 'action View.t list
  -> 'action View.t

(** Settings content may contain [Form.field] or other ordinary views. *)
val settings_group
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> title:string
  -> ?description:string
  -> 'action View.t list
  -> 'action View.t

module Description : sig
  type 'action t

  val create : key:Key.t -> term:string -> definition:'action View.t -> 'action t
end

(** Entry keys must be unique, as for ordinary keyed siblings. Term/definition
    semantic roles are retained in both horizontal and stacked layouts. *)
val description_list
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?stacked:bool
  -> 'action Description.t list
  -> 'action View.t

val empty_state
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?icon:'action View.t
  -> title:string
  -> ?description:string
  -> ?actions:'action View.t
  -> unit
  -> 'action View.t

(** [live] defaults to Polite. Use Off for persistent information, Assertive only
    for urgent changes. Reconciliation emits semantics only when they change. *)
val alert
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> ?live:Accessibility.Live.t
  -> ?icon:'action View.t
  -> title:string
  -> ?actions:'action View.t
  -> 'action View.t list
  -> 'action View.t

val banner
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> ?live:Accessibility.Live.t
  -> ?icon:'action View.t
  -> title:string
  -> ?actions:'action View.t
  -> 'action View.t list
  -> 'action View.t

(** Display only: these labels do not register shortcuts or commands. The caller
    supplies platform-appropriate, already formatted key names. *)
val shortcut_label
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> string list
  -> 'action View.t

(** A structural bar, not a live region by default. Individual status content may
    carry explicit live semantics. Slot contents keep their own native actions.
    [center] fills the space between the ends: its content is centered when both
    ends exist, end-aligned with only [leading], and start-aligned otherwise.
    This is centering in the remaining space, not the whole window. Adding or
    removing a slot preserves controls in the other slots. Omitting [center]
    preserves the two-region layout. *)
val status_bar
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?leading:'action View.t
  -> ?center:'action View.t
  -> ?trailing:'action View.t
  -> unit
  -> 'action View.t

val attachment
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?preview:'action View.t
  -> name:string
  -> ?detail:string
  -> ?actions:'action View.t
  -> unit
  -> 'action View.t

(** Document/asset registration and streaming state remain application-owned.
    These slots also accept native Markdown/code/diff views. *)
val message
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?avatar:'action View.t
  -> author:string
  -> ?detail:string
  -> ?actions:'action View.t
  -> ?footer:'action View.t
  -> 'action View.t
  -> 'action View.t

val bubble
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> ?tone:Tone.t
  -> 'action View.t
  -> 'action View.t

val tool_result
  :  Appearance.t
  -> ?key:Key.t
  -> ?style:Style.t
  -> title:string
  -> ?status:'action View.t
  -> ?actions:'action View.t
  -> ?footer:'action View.t
  -> 'action View.t
  -> 'action View.t

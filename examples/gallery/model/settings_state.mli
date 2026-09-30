open Core
module S = Gpuio.Settings
module N = Gpuio.Number_input

(** Example application data, owned outside all transient Settings rows. *)
module Field : sig
  type t =
    | Name
    | Notifications
    | Reports
    | Budget
    | Region
    | Model
    | Custom
    | Locked
    | Feature of int
  [@@deriving equal, compare, sexp_of]

  val id : t -> S.Item_id.t
  val of_id : S.Item_id.t -> t option
end

type t [@@deriving equal]

val initial : t
val catalog : t -> S.t
val name : t -> string
val notifications : t -> bool
val reports : t -> bool
val budget : t -> N.Value.t
val budget_draft : t -> N.Draft.t
val region : t -> int
val model : t -> int
val custom : t -> int
val feature : t -> int -> bool
val locked : t -> bool

module Action : sig
  type t =
    | Navigate of S.Request.t
    | Name of string
    | Toggle_notifications
    | Toggle_reports
    | Budget of N.Value.t * N.Draft.t
    | Region of int
    | Model of int
    | Custom
    | Toggle_feature of int
    | Toggle_lock
    | Reset of Field.t
end

(** Reduce queued intents against latest data/policy. Native editor resets must
    complete their guarded command before applying [Reset]; this pure model does
    not send commands or perform I/O. Invalid field values return errors. *)
val apply : t -> Action.t -> t Or_error.t

val reset_targets : t -> S.Reset_scope.t -> Field.t list
val can_reset : t -> Field.t -> bool

(** A versioned preview export of committed application values. Numeric
    draft, native selection/history, search/navigation and busy/error state are
    intentionally excluded. The writer enforces the 64 KiB file limit without discarding larger editor
    drafts. This is an example format, not a framework format. *)
val encode : t -> string

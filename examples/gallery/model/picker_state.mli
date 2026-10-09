(** Application-owned destination picker. Open intent and actual visibility are
    separate: clipping must not erase an accepted controlled-open preference. *)
type t [@@deriving equal]

module Action : sig
  type t =
    | Request of Gpuio.Choice_picker.Request.t
    | Request_open of bool
    | Observe of Gpuio.Choice_picker.Visibility.t
    | Toggle_permission
    | Reset
end

val initial : t
val apply : t -> Action.t -> t
val config : t -> Gpuio.Choice_picker.Config.t
val can_open : t -> bool
val requested_open : t -> bool
val visible : t -> bool
val selected_label : t -> string

(** Immutable data only; the native list mounts rows on demand. *)
val workspace_count : int

val workspaces : Gpuio.Choice_picker.Collection.t
val first_workspace : Gpuio.Choice_picker.Collection.t
val empty : Gpuio.Choice_picker.Collection.t

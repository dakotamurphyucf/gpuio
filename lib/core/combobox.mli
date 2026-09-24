open Core

(** Contracts for an editable choice control. The native query is independent
    of the application's selected ID; selection never implicitly replaces text. *)
module Filter : sig
  type t =
    | Substring
    | Unfiltered
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** [Substring] matches Unicode lowercase labels against the native query and
      preserves collection order. It does not perform accent folding or Unicode
      normalization. [Unfiltered] displays the supplied options, permitting
      application-specific search. Applications own asynchronous result ordering.
      Selection must name a member of the complete supplied collection, even if
      native filtering hides it. Empty collections are valid.

      The input is single-line and shares label/disabled state with the choices.
      Enter requests a highlighted choice; it does not submit free text. *)
  val create
    :  label:string
    -> options:Choice.Collection.t
    -> selected:Choice.Id.t option
    -> ?placeholder:string
    -> ?disabled:bool
    -> ?auto_focus:bool
    -> ?filter:Filter.t
    -> unit
    -> t Or_error.t

  val choices : t -> Choice.Config.t
  val filter : t -> Filter.t
end

module Selection : sig
  (** An intent to select an enabled choice, paired with the exact native query
      snapshot at activation. This is not an acknowledgement of application state.
      Selection cannot occur during IME composition. Retain this snapshot's lease
      and revision when conditionally replacing text after accepting the intent. *)
  type t [@@deriving equal, sexp_of]

  val id : t -> Choice.Id.t
  val snapshot : t -> Text_input.Snapshot.t
end

module Event : sig
  type t =
    | Changed of Text_input.Snapshot.t
    | Selected of Selection.t
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val editor_config : Config.t -> Text_input.Config.t

  (** Validate the selection against current application configuration and reject
      composing or multiline snapshots at the native event boundary. *)
  val selection
    :  Config.t
    -> id:Choice.Id.t
    -> snapshot:Text_input.Snapshot.t
    -> Selection.t Or_error.t
end

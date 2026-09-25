open Core

(** Semantic metadata, independent of paint styles and native resource ownership. *)
module Role : sig
  type t =
    | Group
    | Label
    | Link
    | Separator
    | Description_list
    | Term
    | Definition
    | Status
    | Alert
    | Image
    | Heading of int
  [@@deriving equal, sexp_of]
end

module Live : sig
  type t =
    | Off
    | Polite
    | Assertive
  [@@deriving equal, sexp_of]
end

module Field : sig
  type t [@@deriving equal, sexp_of]

  (** Each string is nonempty UTF-8 without NUL, at most 4096 bytes. Use [None]
      for absent help/error. An error marks the control invalid; validation stays
      in application state. Metadata updates do not reset the native editor. *)
  val create
    :  label:string
    -> ?help:string
    -> ?error:string
    -> ?required:bool
    -> unit
    -> t Or_error.t

  val label : t -> string
  val help : t -> string option
  val error : t -> string option
  val is_required : t -> bool
end

type t [@@deriving equal, sexp_of]

(** Heading levels are 1..6. Text has the same bounds as [Field]. The default live
    priority is Polite for Status, Assertive for Alert and Off otherwise. A role
    changes semantics only; [View.with_accessibility] rejects incompatible views. *)
val create
  :  ?role:Role.t
  -> ?label:string
  -> ?description:string
  -> ?live:Live.t
  -> unit
  -> t Or_error.t

(** Apply to a supported native form control, preserving its role and actions.
    The label/help/error become native semantic relationships, not just visible
    text. This is exclusive of general role/live overrides. *)
val field : Field.t -> t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Accessibility_wire.Config.t
end

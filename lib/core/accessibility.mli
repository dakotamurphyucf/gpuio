open Core

module Tree_item : sig
  type t [@@deriving equal, sexp_of]

  (** Root level is 1 (maximum 128); [index] is zero-based in the loaded sibling
      prefix (maximum 99,999). [count = None] means the total is unknown, never
      the number currently mounted. A known count must exceed index, at most
      100,000. Omit [expanded] for leaves; false describes a collapsed branch.
      These are semantic properties, not native interaction ownership. *)
  val create
    :  level:int
    -> index:int
    -> ?count:int
    -> ?expanded:bool
    -> ?selected:bool
    -> ?disabled:bool
    -> ?busy:bool
    -> unit
    -> t Or_error.t
end

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
    | Navigation
    | Tree of bool
    | Tree_item of Tree_item.t
  [@@deriving equal, sexp_of]
end

(** Current member of a related set, distinct from selection, focus or toggled
    state. [None] clears the property. Platforms differ in native support, so a
    current item also requires a localized [description] such as "Current page". *)
module Current : sig
  type t =
    | True
    | Page
    | Step
    | Location
    | Date
    | Time
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

(** [Tree multiple] marks a virtual-list root; [Tree_item item] marks a container
    row. These role overrides do not implement keyboard/tree behavior by themselves.
    Heading levels are 1..6. Text has the same bounds as [Field]. The default live
    priority is Polite for Status, Assertive for Alert and Off otherwise. A role
    changes semantics only; [View.with_accessibility] rejects incompatible views. *)
val create
  :  ?role:Role.t
  -> ?label:string
  -> ?description:string
  -> ?live:Live.t
  -> ?current:Current.t
  -> unit
  -> t Or_error.t

(** Apply to a supported native form control, preserving its role and actions.
    The label/help/error become native semantic relationships, not just visible
    text. This is exclusive of general role/live overrides. *)
val field : Field.t -> t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Accessibility_wire.Config.t
end

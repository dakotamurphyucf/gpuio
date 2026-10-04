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

module Option_item : sig
  type t [@@deriving equal, sexp_of]

  (** [index] is zero-based in logical option order, at most 999,999. A known
      [count] exceeds index and is at most 1,000,000; unknown count is [None].
      Neither value describes only the mounted subset. Selection is independent
      of the cursor/focus. Metadata alone does not install interaction handlers. *)
  val create
    :  index:int
    -> ?count:int
    -> ?selected:bool
    -> ?disabled:bool
    -> unit
    -> t Or_error.t
end

module Table_info : sig
  type t [@@deriving equal, sexp_of]

  (** Logical totals, including headers/footers. Unknown totals are omitted;
      zero is a known empty total. At most 1,000,000 rows and 1,024 columns. *)
  val create : ?rows:int -> ?columns:int -> unit -> t Or_error.t
end

module Table_cell : sig
  type t [@@deriving equal, sexp_of]

  (** Zero-based logical indices. [row] is at most 999,999. [column] and the
      positive [column_span] must fit within 1,024 columns. These properties
      describe geometry; they do not lay out a View or add interaction. *)
  val create : row:int -> column:int -> ?column_span:int -> unit -> t Or_error.t
end

module Orientation : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
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
    | Toolbar of Orientation.t
    | Radio_group of Orientation.t
    | Log
    | List_box of bool
    | Option_item of Option_item.t
    | Table of Table_info.t
    | Row_group
    | Table_row of int
    | Table_cell of Table_cell.t
    | Column_header of Table_cell.t
    | Row_header of Table_cell.t
    | Caption
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
    [List_box multiple] and [Option_item item] likewise mark managed lists and
    container options without installing input behavior.
    [Toolbar orientation] and [Radio_group orientation] mark ordinary containers;
    orientation is semantic
    metadata, not layout or roving keyboard behavior. Heading levels are 1..6.
    [Log] marks an ordinary container or managed list as an ordered transcript.
    It does not retain virtualized history or implement announcements itself.
    Set [live=Off] when streaming fragments should not announce independently;
    applications can announce completed messages through a separate Status.
    Text has the same bounds as [Field]. The default live
    priority is Polite for Status and Log, Assertive for Alert and Off otherwise. A role
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

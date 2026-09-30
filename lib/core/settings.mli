open Core

(** Pure settings catalog, filtering, navigation and reset intent. Application
    values, native editors, persistence and row lifetimes are owned separately.
    This module alone does not render a settings application. *)
module type Id = sig
  type t [@@deriving equal, compare, sexp_of]

  (** Stable, case-sensitive, 1..256 UTF-8 bytes without NUL. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Page_id : Id
module Group_id : Id
module Item_id : Id

module Query : sig
  type t [@@deriving equal, sexp_of]

  val empty : t

  (** Up to 1024 UTF-8 bytes without NUL. Matching is a locale-independent Unicode
      lowercase substring, with no normalization or trimming. Empty matches all. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Reset : sig
  type t =
    | Unavailable
    | Clean
    | Dirty
  [@@deriving equal, sexp_of]
end

module Layout : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Item : sig
  type t [@@deriving equal, sexp_of]

  (** Title: 1..4096 bytes; optional description: up to 16384 bytes; at most
      64 nonempty keywords of up to 256 bytes each. All text is UTF-8 without NUL.
      Cached lowercase search text is bounded to 64 KiB per item. *)
  val create
    :  id:Item_id.t
    -> title:string
    -> ?description:string
    -> ?keywords:string list
    -> ?layout:Layout.t
    -> ?disabled:bool
    -> ?reset:Reset.t
    -> unit
    -> t Or_error.t

  (** A full custom row has no built-in label. Search uses its keywords only. *)
  val custom
    :  id:Item_id.t
    -> ?keywords:string list
    -> ?disabled:bool
    -> ?reset:Reset.t
    -> unit
    -> t Or_error.t

  val id : t -> Item_id.t
  val title : t -> string option
  val description : t -> string option
  val keywords : t -> string list
  val layout : t -> Layout.t
  val is_disabled : t -> bool
  val reset : t -> Reset.t
  val with_disabled : t -> bool -> t
  val with_reset : t -> Reset.t -> t
  val matches : t -> Query.t -> bool
end

module Group : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  id:Group_id.t
    -> ?title:string
    -> ?description:string
    -> Item.t list
    -> t Or_error.t

  val id : t -> Group_id.t
  val title : t -> string option
  val description : t -> string option
  val items : t -> Item.t list
end

module Page : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  id:Page_id.t
    -> title:string
    -> ?description:string
    -> ?default_open:bool
    -> ?resettable:bool
    -> Group.t list
    -> t Or_error.t

  val id : t -> Page_id.t
  val title : t -> string
  val description : t -> string option
  val groups : t -> Group.t list
  val is_resettable : t -> bool
end

module Selection : sig
  type t = private
    { page : Page_id.t
    ; group : Group_id.t option
    }
  [@@deriving equal, sexp_of]

  val create : ?group:Group_id.t -> Page_id.t -> t
end

module Request : sig
  type t =
    | Search of Query.t
    | Select of Selection.t
    | Toggle_page of Page_id.t
  [@@deriving sexp_of]
end

module Reset_scope : sig
  type t =
    | Matching_page of Page_id.t
    | Whole_page of Page_id.t
    | Matching_group of Group_id.t
    | Item of Item_id.t
  [@@deriving sexp_of]
end

type t [@@deriving equal, sexp_of]

(** At most 128 pages, 2048 groups, 32768 items and 8 MiB of catalog text,
    including IDs and cached search text. IDs are unique throughout each kind.
    Empty catalogs/groups/pages are valid; filtered results omit empty groups/pages.
    An explicit selection must exist in the unfiltered catalog. *)
val create : ?selected:Selection.t -> Page.t list -> t Or_error.t

val pages : t -> Page.t list
val query : t -> Query.t
val preferred_selection : t -> Selection.t option
val filtered_pages : t -> Page.t list

(** Preserve the preferred matching page/group; otherwise show its page without
    a hidden group selection, or the first matching page. The preference is not
    overwritten by search, so clearing search restores it. *)
val selection : t -> Selection.t option

val expanded_pages : t -> Page_id.t list
val find_item : t -> Item_id.t -> Item.t option
val select : t -> Selection.t -> t Or_error.t

(** Replace metadata atomically. Keep surviving preferences/expansion, clear a
    removed page preference, downgrade a removed group to its surviving page,
    and apply default_open only to newly added pages. Never infer positional IDs. *)
val with_pages : t -> Page.t list -> t Or_error.t

(** Reduce against current metadata. Ignore stale/nonmatching selection or
    expansion requests; search never mutates application values. *)
val apply_request : t -> Request.t -> t

(** Compute ordered IDs only, against the latest metadata. Matching scopes use
    the current search and effective page; a request for an old page returns [].
    Whole_page explicitly includes filtered-out items. All
    scopes skip disabled, clean or unavailable fields and pages with resettable
    false. Missing scopes return []. No callback, native edit or I/O runs here.
    Callers must resolve the intent again after any asynchronous confirmation;
    native editor writes must use the existing revision/lease checks. *)
val reset_targets : t -> scope:Reset_scope.t -> Item_id.t list

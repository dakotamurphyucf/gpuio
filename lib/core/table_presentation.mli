open Core

(** Presentation of native header bands and managed body rows. The native table
    retains geometry, focus, selection and scrolling. These checked styles are
    separate from ordinary rich-header/cell content Views. *)
module Header : sig
  type t [@@deriving equal, sexp_of]

  (** Accepts Base, Hovered, Pressed and Disabled paint/typography declarations.
      Layout, visibility, opacity, scrolling, pointer/focus/disabled policy,
      border widths and text layout overrides are rejected. At most 128
      declarations. Theme colors resolve during reconciliation. *)
  val create : Style.t -> t Or_error.t

  val empty : t
  val style : t -> Style.t
end

module Row : sig
  type t [@@deriving equal, sexp_of]

  (** Header's scope plus Focused and Selected. Selected means native whole-row
      selection, not merely a selected cell. Focused includes a focused descendant
      or the table's active row while its keyboard focus is on the table.

      Base overrides striping but precedes native selection feedback; explicit state
      refinements then override native feedback. State precedence is Selected,
      Focused, Hovered, Pressed, Disabled. Native selection/context outlines stay
      present. Filler rows have no application presentation. *)
  val create : Style.t -> t Or_error.t

  val empty : t
  val style : t -> Style.t
end

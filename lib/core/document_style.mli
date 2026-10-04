open Core

module Part : sig
  type t =
    | Foreground
    | Muted_foreground
    | Link
    | Selection
    | Code_background
    | Border
  [@@deriving compare, equal, sexp_of]
end

module Heading_sizes : sig
  (** Absolute logical-pixel sizes for levels 1..6, each finite in 1..512. *)
  type t [@@deriving equal, sexp_of]

  val create
    :  h1:float
    -> h2:float
    -> h3:float
    -> h4:float
    -> h5:float
    -> h6:float
    -> t Or_error.t
end

module Underline : sig
  type t [@@deriving equal, sexp_of]

  (** Thickness in logical pixels, finite in 0..32. Omitted color uses text color. *)
  val create : ?color:Color.t -> ?wavy:bool -> thickness:float -> unit -> t Or_error.t
end

module Strikethrough : sig
  type t [@@deriving equal, sexp_of]

  val create : ?color:Color.t -> thickness:float -> unit -> t Or_error.t
end

module Inline_code : sig
  type t [@@deriving equal, sexp_of]

  (** Weight is 1..1000; fade-out is finite in 0..1. Omitted fields inherit
      native text defaults. Omitted background uses the document code background.
      Replacing the value removes prior overrides; [italic=false] selects normal. *)
  val create
    :  ?foreground:Color.t
    -> ?background:Color.t
    -> ?font_weight:int
    -> ?italic:bool
    -> ?underline:Underline.t
    -> ?strikethrough:Strikethrough.t
    -> ?fade_out:float
    -> unit
    -> t Or_error.t

  val default : t
end

(** Internal rich Markdown/HTML presentation. Source/code/diff editor presentation
    and the outer View style remain separate. Values own no native resources. *)
type t [@@deriving equal, sexp_of]

(** Colors resolve through the submission theme; duplicate parts reject.
    Paragraph gap is finite in 0..64 rem. Heading base size is finite in 1..512
    logical pixels; [heading_sizes] overrides all six derived sizes when supplied.

    Part styles accept Base-only background/foreground/opacity, borders/radii/
    shadows, font size/family/weight/alignment/line-height/decoration, width and
    min/max-width, padding and margins. Interaction, visibility, positioning,
    clipping, scrolling and fixed-height overrides reject. At most 512 normalized
    declarations across the four parts. These are reader presentation refinements,
    not independent focus/selection owners. Omitted fields restore native defaults.

    Changes retain canonical source, parser work and logical selection, but may
    invalidate measured native/managed row heights. Explicit [Selection] wins over
    inherited View selection color for rich content. *)
val create
  :  ?colors:(Part.t * Color.t) list
  -> ?paragraph_gap_rem:float
  -> ?heading_base_font_size:float
  -> ?heading_sizes:Heading_sizes.t
  -> ?inline_code:Inline_code.t
  -> ?code_block:Style.t
  -> ?table:Style.t
  -> ?table_head:Style.t
  -> ?table_cell:Style.t
  -> unit
  -> t Or_error.t

val default : t

module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Wire.Document_style.t Or_error.t
end

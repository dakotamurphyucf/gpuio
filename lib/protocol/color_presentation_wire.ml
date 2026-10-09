open Core

module Panel = struct
  type t =
    | Palette
    | Channels
  [@@deriving bin_io, equal, sexp_of]
end

module Panels = struct
  type t =
    | All
    | Tabs of
        { palette_label : string
        ; channels_label : string
        ; initial : Panel.t
        }
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | All -> true
    | Tabs { palette_label; channels_label; initial = _ } ->
      List.for_all [ palette_label; channels_label ] ~f:(fun label ->
        Color_input_wire.valid_label label 256)
  ;;
end

module Section = struct
  type t =
    { featured : bool
    ; label : string
    ; count : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Color_input_wire.valid_label t.label 256 && Int64.(t.count > 0L && t.count <= 256L)
  ;;
end

type t =
  { sections : Section.t list
  ; swatch_size : float
  ; featured_size : float
  ; swatch_gap : float
  ; section_gap : float
  ; swatch_radius : float
  ; outline_width : float
  ; channel_height : float
  ; control_gap : float
  ; padding : float
  ; selected_border : int64 option
  ; hover_border : int64 option
  ; panels : Panels.t
  }
[@@deriving bin_io, equal, sexp_of]

let default =
  { sections = []
  ; swatch_size = 28.
  ; featured_size = 36.
  ; swatch_gap = 6.
  ; section_gap = 10.
  ; swatch_radius = 5.
  ; outline_width = 2.
  ; channel_height = 28.
  ; control_gap = 10.
  ; padding = 10.
  ; selected_border = None
  ; hover_border = None
  ; panels = All
  }
;;

let valid_sections sections =
  List.length sections <= 32
  && List.for_alli sections ~f:(fun index section ->
    Section.valid section && ((not section.featured) || index = 0))
  && Int64.(List.sum (module Int64) sections ~f:(fun s -> s.Section.count) <= 256L)
;;

let valid t =
  let bounded value low high =
    Float.is_finite value && Float.(value >= low && value <= high)
  in
  valid_sections t.sections
  && Panels.valid t.panels
  && List.for_all [ t.swatch_size; t.featured_size; t.channel_height ] ~f:(fun n ->
    bounded n 16. 128.)
  && List.for_all [ t.swatch_gap; t.section_gap; t.control_gap; t.padding ] ~f:(fun n ->
    bounded n 0. 64.)
  && List.for_all [ t.swatch_radius; t.outline_width ] ~f:(fun n ->
    bounded n 0. (Float.min t.swatch_size t.featured_size /. 2.))
  && List.for_all
       [ t.selected_border; t.hover_border ]
       ~f:(Option.for_all ~f:(fun n -> Int64.(n >= 0L && n <= 0xffffffffL)))
;;

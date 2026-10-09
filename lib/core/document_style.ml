open Core
module W = Gpuio_protocol.Wire.Document_style
module Part = W.Part

let bounded value ~min:minimum ~max:maximum =
  Float.is_finite value && Float.(value >= minimum && value <= maximum)
;;

module Heading_sizes = struct
  type t = W.Heading_sizes.t [@@deriving equal, sexp_of]

  let create ~h1 ~h2 ~h3 ~h4 ~h5 ~h6 =
    if List.for_all [ h1; h2; h3; h4; h5; h6 ] ~f:(bounded ~min:1. ~max:512.)
    then Ok { W.Heading_sizes.h1; h2; h3; h4; h5; h6 }
    else Or_error.error_string "heading sizes must be finite in 1..512 pixels"
  ;;
end

module Underline = struct
  type t =
    { color : Color.t option
    ; thickness : float
    ; wavy : bool
    }
  [@@deriving equal, sexp_of]

  let create ?color ?(wavy = false) ~thickness () =
    if bounded thickness ~min:0. ~max:32.
    then Ok { color; thickness; wavy }
    else Or_error.error_string "underline thickness must be finite in 0..32 pixels"
  ;;
end

module Strikethrough = struct
  type t =
    { color : Color.t option
    ; thickness : float
    }
  [@@deriving equal, sexp_of]

  let create ?color ~thickness () =
    if bounded thickness ~min:0. ~max:32.
    then Ok { color; thickness }
    else Or_error.error_string "strikethrough thickness must be finite in 0..32 pixels"
  ;;
end

module Inline_code = struct
  type t =
    { foreground : Color.t option
    ; background : Color.t option
    ; font_weight : int option
    ; italic : bool option
    ; underline : Underline.t option
    ; strikethrough : Strikethrough.t option
    ; fade_out : float option
    }
  [@@deriving equal, sexp_of]

  let create
        ?foreground
        ?background
        ?font_weight
        ?italic
        ?underline
        ?strikethrough
        ?fade_out
        ()
    =
    if
      Option.for_all font_weight ~f:(fun n -> n >= 1 && n <= 1000)
      && Option.for_all fade_out ~f:(bounded ~min:0. ~max:1.)
    then
      Ok
        { foreground
        ; background
        ; font_weight
        ; italic
        ; underline
        ; strikethrough
        ; fade_out
        }
    else Or_error.error_string "inline code weight/fade is out of bounds"
  ;;

  let default = create () |> Or_error.ok_exn
end

type t =
  { colors : (Part.t * Color.t) list
  ; paragraph_gap_rem : float option
  ; heading_base_font_size : float option
  ; heading_sizes : Heading_sizes.t option
  ; inline_code : Inline_code.t
  ; code_block : Style.t
  ; table : Style.t
  ; table_head : Style.t
  ; table_cell : Style.t
  }
[@@deriving equal, sexp_of]

let create
      ?(colors = [])
      ?paragraph_gap_rem
      ?heading_base_font_size
      ?heading_sizes
      ?(inline_code = Inline_code.default)
      ?(code_block = Style.empty)
      ?(table = Style.empty)
      ?(table_head = Style.empty)
      ?(table_cell = Style.empty)
      ()
  =
  let open Or_error.Let_syntax in
  let%bind () =
    if
      List.length colors <= 6
      && (not (List.contains_dup (List.map colors ~f:fst) ~compare:Part.compare))
      && Option.for_all paragraph_gap_rem ~f:(bounded ~min:0. ~max:64.)
      && Option.for_all heading_base_font_size ~f:(bounded ~min:1. ~max:512.)
    then Ok ()
    else
      Or_error.error_string
        "duplicate document colors or invalid paragraph/heading metrics"
  in
  let styles = [ code_block; table; table_head; table_cell ] in
  let%bind () =
    List.map styles ~f:(fun style ->
      Style.Expert.validate_scope
        style
        ~states:[ Base ]
        ~properties:
          [ Background
          ; Foreground
          ; Opacity
          ; Border_color
          ; Border_style
          ; Border_top_width
          ; Border_right_width
          ; Border_bottom_width
          ; Border_left_width
          ; Top_left_radius
          ; Top_right_radius
          ; Bottom_left_radius
          ; Bottom_right_radius
          ; Shadows
          ; Font_size
          ; Font_family
          ; Font_weight
          ; Text_align
          ; Line_height
          ; Text_decoration
          ; Width
          ; Min_width
          ; Max_width
          ; Padding_top
          ; Padding_right
          ; Padding_bottom
          ; Padding_left
          ; Margin_top
          ; Margin_right
          ; Margin_bottom
          ; Margin_left
          ])
    |> Or_error.all_unit
  in
  let%map () =
    if List.sum (module Int) styles ~f:Style.Expert.declaration_count <= 512
    then Ok ()
    else Or_error.error_string "document part styles exceed 512 declarations"
  in
  { colors = List.sort colors ~compare:(fun (a, _) (b, _) -> Part.compare a b)
  ; paragraph_gap_rem
  ; heading_base_font_size
  ; heading_sizes
  ; inline_code
  ; code_block
  ; table
  ; table_head
  ; table_cell
  }
;;

let default = create () |> Or_error.ok_exn

module Expert = struct
  let option value ~f =
    match value with
    | None -> Ok None
    | Some value -> Or_error.map (f value) ~f:Option.some
  ;;

  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let resolve value =
      let%map rgba = Theme.resolve theme value in
      Gpuio_protocol.Wire.Color.Rgba rgba
    in
    let color value = option value ~f:resolve in
    let%bind colors =
      List.map t.colors ~f:(fun (part, c) ->
        let%map c = resolve c in
        part, c)
      |> Or_error.all
    in
    let%bind foreground = color t.inline_code.foreground in
    let%bind background = color t.inline_code.background in
    let%bind underline =
      option t.inline_code.underline ~f:(fun u ->
        let%map color = color u.color in
        { W.Underline.color; thickness = u.thickness; wavy = u.wavy })
    in
    let%bind strikethrough =
      option t.inline_code.strikethrough ~f:(fun u ->
        let%map color = color u.color in
        { W.Strikethrough.color; thickness = u.thickness })
    in
    let%bind code_block = Style.Expert.to_wire t.code_block ~theme in
    let%bind table = Style.Expert.to_wire t.table ~theme in
    let%bind table_head = Style.Expert.to_wire t.table_head ~theme in
    let%map table_cell = Style.Expert.to_wire t.table_cell ~theme in
    { W.colors
    ; paragraph_gap_rem = t.paragraph_gap_rem
    ; heading_base_font_size = t.heading_base_font_size
    ; heading_sizes = t.heading_sizes
    ; inline_code =
        { W.Inline_code.foreground
        ; background
        ; font_weight = Option.map t.inline_code.font_weight ~f:Int64.of_int
        ; italic = t.inline_code.italic
        ; underline
        ; strikethrough
        ; fade_out = t.inline_code.fade_out
        }
    ; code_block
    ; table
    ; table_head
    ; table_cell
    }
  ;;
end

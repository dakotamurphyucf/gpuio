open Core
module Field = Accessibility.Field

module Layout = struct
  type t =
    | Vertical
    | Horizontal
  [@@deriving equal, sexp_of]
end

let key = Key.of_string_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

let field
      config
      ?key:field_key
      ?(style = Style.empty)
      ?(layout = Layout.Vertical)
      ?(label_style = Style.empty)
      ?(help_style = Style.empty)
      ?(error_style = Style.empty)
      ~control
      ()
  =
  let open Style.Property in
  let%map.Or_error control =
    View.with_accessibility control (Accessibility.field config)
  in
  let create = Style.create_exn in
  let label =
    let text = Field.label config ^ if Field.is_required config then " *" else "" in
    View.text
      ~key:(key "label")
      ~style:
        (Style.merge
           [ create
               ([ Font_weight 600; Shrink 0. ]
                @
                match layout with
                | Vertical -> []
                | Horizontal -> [ Width (px 160.) ])
           ; label_style
           ])
      text
  in
  let annotation name appearance text =
    Option.map text ~f:(fun text -> View.text ~key:(key name) ~style:appearance text)
  in
  let help =
    annotation
      "help"
      (Style.merge
         [ create [ Foreground (Color.token_exn "muted"); Font_size 12. ]; help_style ])
      (Field.help config)
  in
  let error =
    annotation
      "error"
      (Style.merge
         [ create [ Foreground (Color.rgb_exn 0xdf5566); Font_size 12. ]; error_style ])
      (Field.error config)
  in
  let content =
    View.column
      ~key:(key "content")
      ~style:(create [ Grow 1.; Min_width (px 0.); Gap (px 4.) ])
      (View.column ~key:(key "control") [ control ] :: List.filter_opt [ help; error ])
  in
  let style =
    Style.merge [ create [ Width full; Gap (px 8.); Min_width (px 0.) ]; style ]
  in
  match layout with
  | Vertical -> View.column ?key:field_key ~style [ label; content ]
  | Horizontal -> View.row ?key:field_key ~style [ label; content ]
;;

module Size = struct
  type t =
    | XSmall
    | Small
    | Medium
    | Large
  [@@deriving equal, sexp_of]

  let spacing = function
    | XSmall -> 4., 8., 12.
    | Small -> 6., 12., 13.
    | Medium -> 8., 16., 14.
    | Large -> 12., 24., 16.
  ;;
end

let validate_label_width width =
  match Length.Expert.to_wire width with
  | Auto -> Or_error.error_string "form label width must be definite"
  | Px value | Percent value ->
    if Float.(value < 0.)
    then Or_error.error_string "form label width must be nonnegative"
    else Ok ()
;;

module Item = struct
  type 'action t =
    { key : Key.t
    ; column : Style.Grid_location.Axis.t
    ; layout : Layout.t option
    ; size : Size.t option
    ; label_width : Length.t option
    ; label_indent : bool option
    ; alignment : Style.Align.t option
    ; style : Style.t
    ; label_style : Style.t
    ; description_style : Style.t
    ; error_style : Style.t
    ; required : bool
    ; label : 'action View.t option
    ; description : 'action View.t option
    ; error : 'action View.t option
    ; content : 'action View.t list
    }

  let create
        ~key
        ?(column = Style.Grid_location.Axis.auto)
        ?layout
        ?size
        ?label_width
        ?label_indent
        ?alignment
        ?(style = Style.empty)
        ?(label_style = Style.empty)
        ?(description_style = Style.empty)
        ?(error_style = Style.empty)
        ?(required = false)
        ?label
        ?description
        ?error
        content
    =
    let%map.Or_error () =
      Option.value_map label_width ~default:(Ok ()) ~f:validate_label_width
    in
    { key
    ; column
    ; layout
    ; size
    ; label_width
    ; label_indent
    ; alignment
    ; style
    ; label_style
    ; description_style
    ; error_style
    ; required
    ; label
    ; description
    ; error
    ; content
    }
  ;;

  let of_field
        config
        ~key
        ?column
        ?layout
        ?size
        ?label_width
        ?label_indent
        ?alignment
        ?style
        ?label_style
        ?description_style
        ?error_style
        ?label
        ?description
        ?error
        ~control
        ()
    =
    let open Or_error.Let_syntax in
    let%bind control = View.with_accessibility control (Accessibility.field config) in
    let label = Option.value label ~default:(View.text (Field.label config)) in
    let description =
      Option.first_some description (Option.map (Field.help config) ~f:View.text)
    in
    let error = Option.first_some error (Option.map (Field.error config) ~f:View.text) in
    create
      ~key
      ?column
      ?layout
      ?size
      ?label_width
      ?label_indent
      ?alignment
      ?style
      ?label_style
      ?description_style
      ?error_style
      ~required:(Field.is_required config)
      ~label
      ?description
      ?error
      [ control ]
  ;;
end

(* Match the pinned native in-flow grid normalization. Keep signed lines until
   this point so a collection's current explicit column count determines them. *)
let validate_column column ~columns =
  let module G = Style.Grid_location in
  let line value =
    let value = G.Line.to_int value in
    if value > 0 then value - 1 else value + columns + 1
  in
  let valid_range first last = first >= 0 && first < last && last <= columns in
  let valid =
    match G.Axis.start column, G.Axis.end_ column with
    | Line first, Line last ->
      let first, last = line first, line last in
      if first = last
      then valid_range first (first + 1)
      else valid_range (Int.min first last) (Int.max first last)
    | Line first, Span count ->
      let first = line first in
      valid_range first (first + G.Span.to_int count)
    | Line first, Auto ->
      let first = line first in
      valid_range first (first + 1)
    | Span count, Line last ->
      let last = line last in
      valid_range (last - G.Span.to_int count) last
    | Auto, Line last ->
      let last = line last in
      valid_range (last - 1) last
    | Span count, (Auto | Span _) | Auto, Span count -> G.Span.to_int count <= columns
    | Auto, Auto -> true
  in
  if valid
  then Ok ()
  else Or_error.error_string "form item placement exceeds column count"
;;

let render_item
      (item : _ Item.t)
      ~layout
      ~size
      ~label_width
      ~label_indent
      ~alignment
      ~label_style
      ~description_style
      ~error_style
  =
  let open Style.Property in
  let style = Style.create_exn in
  let layout = Option.value item.layout ~default:layout in
  let size = Option.value item.size ~default:size in
  let label_width = Option.value item.label_width ~default:label_width in
  let label_indent = Option.value item.label_indent ~default:label_indent in
  let alignment = Option.value item.alignment ~default:alignment in
  let horizontal = Layout.equal layout Horizontal in
  let gap, _, font_size = Size.spacing size in
  let label =
    if Option.is_some item.label || item.required || (horizontal && label_indent)
    then
      Some
        (View.row
           ~key:(key "label")
           ~style:
             (Style.merge
                [ style
                    [ Min_width (px 0.)
                    ; Gap (px 4.)
                    ; Font_size font_size
                    ; Font_weight 600
                    ; Align_items Center
                    ]
                ; style (if horizontal then [] else [ Width full ])
                ; label_style
                ; item.label_style
                ; style (if horizontal then [ Width label_width; Shrink 0. ] else [])
                ])
           (List.filter_opt
              [ Option.map item.label ~f:(fun label ->
                  View.column
                    ~key:(key "label-content")
                    ~style:(style [ Grow 1.; Min_width (px 0.) ])
                    [ label ])
              ; (if item.required
                 then
                   Some
                     (View.text
                        ~key:(key "required")
                        ~style:(style [ Foreground (Color.rgb_exn 0xdf5566) ])
                        "*")
                 else None)
              ]))
    else None
  in
  let annotation name default shared custom content =
    Option.map content ~f:(fun view ->
      View.column
        ~key:(key name)
        ~style:
          (Style.merge
             [ style
                 [ Font_size (font_size -. 2.); Foreground default; Min_width (px 0.) ]
             ; shared
             ; custom
             ])
        [ view ])
  in
  let description =
    annotation
      "description"
      (Color.token_exn "muted")
      description_style
      item.description_style
      item.description
  in
  let error =
    annotation "error" (Color.rgb_exn 0xdf5566) error_style item.error_style item.error
  in
  let content =
    View.column
      ~key:(key "content")
      ~style:
        (style
           ([ Min_width (px 0.); Grow 1.; Gap (px (gap /. 2.)) ]
            @ if horizontal then [] else [ Width full ]))
      (View.column
         ~key:(key "controls")
         ~style:(style [ Min_width (px 0.); Gap (px (gap /. 2.)) ])
         item.content
       :: List.filter_opt [ description; error ])
  in
  View.column
    ~key:item.key
    ~style:
      (Style.merge
         [ style [ Min_width (px 0.); Gap (px gap); Align_items alignment ]
         ; item.style
         ; style
             [ Position Relative
             ; Direction (if horizontal then Row else Column)
             ; Grid_location (Style.Grid_location.create ~column:item.column ())
             ]
         ])
    (List.filter_opt [ label; Some content ])
;;

let create
      ?key:form_key
      ?(columns = 1)
      ?(layout = Layout.Vertical)
      ?(size = Size.Medium)
      ?(label_width = px 160.)
      ?(label_indent = true)
      ?(alignment = Style.Align.Start)
      ?(style = Style.empty)
      ?(grid_style = Style.empty)
      ?(label_style = Style.empty)
      ?(description_style = Style.empty)
      ?(error_style = Style.empty)
      ?(footer_style = Style.empty)
      ?footer
      items
  =
  let open Or_error.Let_syntax in
  let%bind () =
    if columns < 1 || columns > 1024
    then Or_error.error_string "form columns must be in 1..1024"
    else Ok ()
  in
  let%bind () = validate_label_width label_width in
  let%bind () =
    match
      List.find_a_dup (List.map items ~f:(fun item -> item.Item.key)) ~compare:Key.compare
    with
    | None -> Ok ()
    | Some key -> Or_error.errorf "duplicate form item key: %s" (Key.to_string key)
  in
  let%bind () =
    Or_error.all_unit
      (List.map items ~f:(fun item ->
         Or_error.tag_arg
           (validate_column item.Item.column ~columns)
           "form item"
           item.key
           Key.sexp_of_t))
  in
  let open Style.Property in
  let make_style = Style.create_exn in
  let _, gap, _ = Size.spacing size in
  let grid =
    View.column
      ~key:(key "grid")
      ~style:
        (Style.merge
           [ make_style [ Width full; Min_width (px 0.); Gap (px gap); Align_items Start ]
           ; grid_style
           ; make_style [ Display Grid; Grid_columns columns ]
           ])
      (List.map
         items
         ~f:
           (render_item
              ~layout
              ~size
              ~label_width
              ~label_indent
              ~alignment
              ~label_style
              ~description_style
              ~error_style))
  in
  let footer =
    Option.map footer ~f:(fun footer ->
      View.row
        ~key:(key "footer")
        ~style:
          (Style.merge
             [ make_style [ Width full; Min_width (px 0.); Justify_content End ]
             ; footer_style
             ])
        [ footer ])
  in
  Ok
    (View.column
       ?key:form_key
       ~style:
         (Style.merge
            [ make_style [ Width full; Min_width (px 0.); Gap (px gap) ]; style ])
       (grid :: Option.to_list footer))
;;

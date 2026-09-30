open Core

let px = Length.px_exn
let full = Length.percent_exn 100.
let key = Key.of_string_exn
let style = Style.create_exn

let control
      field
      ?key:root_key
      ?style:root_style
      ?(help_style = Style.empty)
      ?(error_style = Style.empty)
      ?(size = Presentation.Size.Medium)
      ~layout
      ~control
      ()
  =
  let%map.Or_error control =
    View.with_accessibility control (Accessibility.field field)
  in
  let font, gap =
    match size with
    | Small -> 12., 4.
    | Medium -> 14., 6.
    | Large -> 16., 8.
  in
  let annotation name base refinement text =
    Option.map text ~f:(fun text ->
      View.text ~key:(key name) ~style:(Style.merge [ base; refinement ]) text)
  in
  let help =
    annotation
      "help"
      (style [ Opacity 0.7; Font_size 12. ])
      help_style
      (Accessibility.Field.help field)
  in
  let error =
    annotation
      "error"
      (style [ Foreground (Color.rgb_exn 0xdf5566); Font_size 12. ])
      error_style
      (Accessibility.Field.error field)
  in
  View.column
    ?key:root_key
    ~style:
      (Style.merge
         [ style
             [ Width
                 (match layout with
                  | Settings.Layout.Horizontal -> px 256.
                  | Vertical -> full)
             ; Max_width full
             ; Min_width (px 0.)
             ; Font_size font
             ; Gap (px gap)
             ]
         ; Option.value root_style ~default:Style.empty
         ])
    (View.column ~key:(key "control") [ control ] :: List.filter_opt [ help; error ])
;;

let switch field ?key ?style ?size ?(disabled = false) ~layout ~checked ~on_toggle () =
  control
    field
    ?key
    ?style
    ?size
    ~layout
    ~control:(View.switch ~disabled ~checked ~on_toggle "")
    ()
;;

let checkbox field ?key ?style ?size ?(disabled = false) ~layout ~checked ~on_toggle () =
  control
    field
    ?key
    ?style
    ?size
    ~layout
    ~control:
      (View.checkbox
         ~disabled
         ~state:(if checked then Checked else Unchecked)
         ~on_toggle
         "")
    ()
;;

module Choices = struct
  type 'a t =
    | T :
        { options : Choice.Collection.t
        ; by_value : ('a, Choice.Id.t, 'cmp) Map.t
        ; by_id : 'a String.Map.t
        }
        -> 'a t

  let create comparator entries =
    let open Or_error.Let_syntax in
    let%bind options = Choice.Collection.create (List.map entries ~f:snd) in
    let%bind by_value =
      Map.of_alist_or_error
        comparator
        (List.map entries ~f:(fun (value, choice) -> value, Choice.id choice))
    in
    let%map by_id =
      String.Map.of_alist_or_error
        (List.map entries ~f:(fun (value, choice) ->
           Choice.Id.to_string (Choice.id choice), value))
    in
    T { options; by_value; by_id }
  ;;

  let select
        (T t)
        ~field
        ?key
        ?style
        ?size
        ?(disabled = false)
        ?appearance
        ~layout
        ~selected
        ~on_select
        ()
    =
    let open Or_error.Let_syntax in
    let%bind selected =
      match selected with
      | None -> Ok None
      | Some value ->
        (match Map.find t.by_value value with
         | Some id -> Ok (Some id)
         | None -> Or_error.error_string "selected settings value is absent from choices")
    in
    let%bind config =
      Choice.Config.create
        ~label:(Accessibility.Field.label field)
        ~options:t.options
        ~selected
        ~disabled
        ()
    in
    control
      field
      ?key
      ?style
      ?size
      ~layout
      ~control:
        (View.select
           ?appearance
           ~config
           ~on_select:(fun id ->
             on_select (Map.find_exn t.by_id (Choice.Id.to_string id)))
           ())
      ()
  ;;
end

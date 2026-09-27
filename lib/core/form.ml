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

open Core
open Gpuio
module P = Presentation
module B = Bonsai.Cont

module Content = struct
  type t =
    | Long
    | Empty
    | Localized
  [@@deriving equal]

  let all = [ Long; Empty; Localized ]

  let name = function
    | Long -> "long"
    | Empty -> "empty"
    | Localized -> "localized"
  ;;

  let text = function
    | Long ->
      "A descriptive workspace item with enough words to wrap across several lines in a \
       narrow panel, while keeping its actions visible and available."
    | Empty -> ""
    | Localized -> "共有ワークスペースの設定 · Résultats et pièces jointes · Grüße 👩‍👩‍👧‍👦"
  ;;
end

module Case = struct
  type t =
    | Label
    | Tag
    | Badge
    | Marker
    | Link
    | Group
    | Settings
    | Description
    | Empty_state
    | Alert
    | Banner
    | Shortcut
    | Status
    | Attachment
    | Message
    | Bubble
    | Tool_result
    | Vertical_field
    | Horizontal_field

  let all =
    [ Label
    ; Tag
    ; Badge
    ; Marker
    ; Link
    ; Group
    ; Settings
    ; Description
    ; Empty_state
    ; Alert
    ; Banner
    ; Shortcut
    ; Status
    ; Attachment
    ; Message
    ; Bubble
    ; Tool_result
    ; Vertical_field
    ; Horizontal_field
    ]
  ;;

  let name = function
    | Label -> "label"
    | Tag -> "tag"
    | Badge -> "badge"
    | Marker -> "marker"
    | Link -> "link"
    | Group -> "group"
    | Settings -> "settings"
    | Description -> "description"
    | Empty_state -> "empty state"
    | Alert -> "alert"
    | Banner -> "banner"
    | Shortcut -> "shortcut"
    | Status -> "status"
    | Attachment -> "attachment"
    | Message -> "message"
    | Bubble -> "bubble"
    | Tool_result -> "tool result"
    | Vertical_field -> "vertical field"
    | Horizontal_field -> "horizontal field"
  ;;
end

let px = Length.px_exn
let style = Style.create_exn
let ok = Or_error.ok_exn

let view case p content ~on_action =
  let text = Content.text content in
  let action = View.button ~on_click:(fun () -> on_action) "Open item" in
  match case with
  | Case.Label -> P.label text
  | Tag -> P.tag p ~leading:(View.text "ID") ~trailing:action text
  | Badge -> P.badge p ~leading:(View.text "ID") text
  | Marker -> P.marker p text
  | Link ->
    P.link
      p
      ~on_click:(fun () -> on_action)
      (if Content.equal content Empty then "Open item" else text)
  | Group -> P.group_box p ~header:(View.text text) ~footer:action []
  | Settings -> P.settings_group p ~title:text ~description:text [ action ]
  | Description ->
    P.description_list
      p
      [ P.Description.create ~key:(Key.of_int 0) ~term:text ~definition:(View.text text) ]
  | Empty_state -> P.empty_state p ~title:text ~description:text ~actions:action ()
  | Alert -> P.alert p ~title:text ~actions:action [ View.text text ]
  | Banner -> P.banner p ~title:text ~actions:action []
  | Shortcut ->
    P.shortcut_label p (if Content.equal content Empty then [] else [ "⌘"; text ])
  | Status -> P.status_bar p ~leading:(P.marker p text) ~trailing:action ()
  | Attachment ->
    P.attachment p ~name:text ~detail:text ~preview:(View.text "MD") ~actions:action ()
  | Message ->
    P.message
      p
      ~author:text
      ~detail:text
      ~actions:action
      ~footer:(View.text text)
      (P.bubble p (View.text text))
  | Bubble -> P.bubble p (View.text text)
  | Tool_result ->
    P.tool_result
      p
      ~title:text
      ~status:(P.badge p "Ready")
      ~actions:action
      (View.text text)
  | Vertical_field | Horizontal_field ->
    let label = if Content.equal content Empty then "Optional field" else text in
    let field =
      Form.Field.create ~label ?help:(Option.some_if (not (String.is_empty text)) text) ()
      |> ok
    in
    Form.field
      field
      ~layout:
        (match case with
         | Horizontal_field -> Horizontal
         | _ -> Vertical)
      ~control:(View.checkbox ~state:Unchecked ~on_toggle:(fun () -> on_action) "Enabled")
      ()
    |> ok
;;

let cases = List.cartesian_product Case.all Content.all |> Array.of_list

let component _window graph =
  let index, set_index = B.state 0 graph in
  let dark, toggle_dark = B.toggle ~default_model:true graph in
  let action_count, set_action_count = B.state 0 graph in
  let open B.Let_syntax in
  let%arr index = index
  and set_index = set_index
  and dark = dark
  and toggle_dark = toggle_dark
  and action_count = action_count
  and set_action_count = set_action_count in
  let case, content = cases.(index) in
  let p = if dark then P.Appearance.dark else P.Appearance.light in
  let open Style.Property in
  View.column
    ~style:
      (style
         [ Width (Length.percent_exn 100.)
         ; Height (Length.percent_exn 100.)
         ; Padding (px 16.)
         ; Gap (px 16.)
         ; Overflow_y Scroll
         ; Background
             (Background.solid (Color.rgb_exn (if dark then 0x131821 else 0xf5f6fa)))
         ; Foreground (Color.rgb_exn (if dark then 0xe5eaf2 else 0x202735))
         ; Font_size 14.
         ])
    [ View.row
        ~style:(style [ Gap (px 8.) ])
        [ View.button
            ~on_click:(fun () -> set_index ((index + 1) % Array.length cases))
            "Next case"
        ; View.button ~on_click:(fun () -> toggle_dark) "Change theme"
        ]
    ; View.text
        (sprintf
           "Case %d / %s / %s / %s"
           index
           (Case.name case)
           (Content.name content)
           (if dark then "dark" else "light"))
    ; view case p content ~on_action:(set_action_count (action_count + 1))
    ; View.text (sprintf "Fixture end · actions %d" action_count)
    ]
;;

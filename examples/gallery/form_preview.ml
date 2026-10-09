open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input
module G = Style.Grid_location

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let key = Key.of_string_exn

let named label children =
  V.column ~style:(style [ Min_width (px 0.) ]) children
  |> fun view ->
  V.with_accessibility view (Accessibility.create ~role:Group ~label () |> ok) |> ok
;;

let component window palette graph =
  let columns, next_columns =
    B.state_machine0
      ~default_model:2
      ~apply_action:(fun _ columns () -> 1 + (columns % 3))
      graph
  in
  let horizontal, toggle_horizontal = B.toggle ~default_model:false graph in
  let invalid, toggle_invalid = B.toggle ~default_model:false graph in
  let reverse, toggle_reverse = B.toggle ~default_model:false graph in
  let indent, toggle_indent = B.toggle ~default_model:true graph in
  let mixed, toggle_mixed = B.toggle ~default_model:false graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let hidden, toggle_hidden = B.toggle ~default_model:false graph in
  let notifications, toggle_notifications = B.toggle ~default_model:true graph in
  let shared, toggle_shared = B.toggle ~default_model:false graph in
  let size, next_size =
    B.state_machine0
      ~default_model:Form.Size.Medium
      ~apply_action:(fun _ size () ->
        match size with
        | XSmall -> Small
        | Small -> Medium
        | Medium -> Large
        | Large -> XSmall)
      graph
  in
  let actions, act =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let editor =
    Editor.create
      window
      ~initial_text:"Northstar"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Form workspace name" ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and columns = columns
  and next_columns = next_columns
  and horizontal = horizontal
  and toggle_horizontal = toggle_horizontal
  and invalid = invalid
  and toggle_invalid = toggle_invalid
  and reverse = reverse
  and toggle_reverse = toggle_reverse
  and indent = indent
  and toggle_indent = toggle_indent
  and mixed = mixed
  and toggle_mixed = toggle_mixed
  and refined = refined
  and toggle_refined = toggle_refined
  and hidden = hidden
  and toggle_hidden = toggle_hidden
  and notifications = notifications
  and toggle_notifications = toggle_notifications
  and shared = shared
  and toggle_shared = toggle_shared
  and size = size
  and next_size = next_size
  and actions = actions
  and act = act
  and editor = editor in
  let name =
    Form.Item.of_field
      (Form.Field.create
         ~label:"Form workspace name"
         ~help:"Visible to your team."
         ~required:true
         ?error:(Option.some_if invalid "A workspace name is required.")
         ()
       |> ok)
      ~key:(key "workspace-name")
      ~column:
        (if mixed
         then G.Axis.span (G.Span.of_int_exn (Int.min columns 2))
         else G.Axis.auto)
      ~style:(style (if hidden then [ Display Hidden ] else []))
      ~label:
        (named
           "Form name label"
           [ Palette.text p "Workspace"
           ; Palette.text p ~size:11. ~muted:true "Your shared space"
           ])
      ~description:
        (named
           "Form name description"
           [ Palette.text p ~size:12. ~muted:true "Visible to your team." ])
      ~control:(Editor.view ~style:(style [ Height (px 38.); Width full ]) editor)
      ()
    |> ok
  in
  let preferences =
    Form.Item.create
      ~key:(key "preferences")
      ~label:(named "Form preference label" [ Palette.text p "Preferences" ])
      ~label_width:(px 96.)
      ~description:
        (Palette.text p ~size:12. ~muted:true "Each control keeps its own state.")
      [ named
          "Form preference content"
          [ V.switch
              ~checked:notifications
              ~on_toggle:toggle_notifications
              "Form notifications"
          ; V.checkbox
              ~state:(if shared then Checked else Unchecked)
              ~on_toggle:toggle_shared
              "Form shared access"
          ]
      ]
    |> ok
  in
  let unlabeled =
    Form.Item.create
      ~key:(key "unlabeled")
      [ named
          "Form unlabeled content"
          [ Palette.button p "Form inspect permissions" (act ()) ]
      ]
    |> ok
  in
  let summary =
    Form.Item.create
      ~key:(key "summary")
      ~column:G.Axis.full
      ~layout:Vertical
      ~size:Small
      ~label:(named "Form summary label" [ Palette.text p "Before you save" ])
      [ named
          "Form summary content"
          [ Palette.text
              p
              ~muted:true
              "Column layout, rich labels and validation can change without replacing \
               your draft. 京都 · 👨‍👩‍👧‍👦"
          ]
      ]
    |> ok
  in
  let entries = [ name; preferences; unlabeled; summary ] in
  let form =
    Form.create
      ~key:(key "workspace-form")
      ~columns
      ~layout:(if horizontal then Horizontal else Vertical)
      ~size
      ~label_width:(px 112.)
      ~label_indent:indent
      ~style:(style [ Foreground (Palette.foreground p) ])
      ~label_style:
        (style
           (if refined
            then [ Background (Background.solid (Palette.border p)); Padding (px 4.) ]
            else []))
      ~description_style:(style [ Foreground (Palette.muted p) ])
      ~footer:
        (named "Form footer content" [ Palette.button p "Form save workspace" (act ()) ])
      (if reverse then List.rev entries else entries)
    |> ok
    |> fun view ->
    V.with_accessibility
      view
      (Accessibility.create ~role:Group ~label:"Workspace form" () |> ok)
    |> ok
  in
  let toggle label checked on_toggle =
    V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
  in
  let size_name =
    match size with
    | XSmall -> "XS"
    | Small -> "S"
    | Medium -> "M"
    | Large -> "L"
  in
  V.column
    ~style:(style [ Gap (px 16.) ])
    [ V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p (sprintf "Form columns: %d" columns) (next_columns ())
        ; Palette.button p ("Form size: " ^ size_name) (next_size ())
        ]
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ toggle "Horizontal form labels" horizontal toggle_horizontal
        ; toggle "Form validation error" invalid toggle_invalid
        ; toggle "Reverse form items" reverse toggle_reverse
        ; toggle "Indent absent form labels" indent toggle_indent
        ; toggle "Mixed form spans" mixed toggle_mixed
        ; toggle "Refine form labels" refined toggle_refined
        ; toggle "Hide form name" hidden toggle_hidden
        ]
    ; form
    ; Palette.text p ~muted:true (sprintf "Form actions: %d" actions)
    ]
;;

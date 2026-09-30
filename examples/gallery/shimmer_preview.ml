open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module S = Text_shimmer

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let enabled, toggle_enabled = B.toggle ~default_model:true graph in
  let playing, toggle_playing = B.toggle ~default_model:false graph in
  let reverse, toggle_reverse = B.toggle ~default_model:false graph in
  let once, toggle_once = B.toggle ~default_model:false graph in
  let compact, toggle_compact = B.toggle ~default_model:false graph in
  let revision, refresh =
    B.state_machine0
      ~default_model:1
      ~apply_action:(fun _ revision () -> revision + 1)
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and enabled = enabled
  and toggle_enabled = toggle_enabled
  and playing = playing
  and toggle_playing = toggle_playing
  and reverse = reverse
  and toggle_reverse = toggle_reverse
  and once = once
  and toggle_once = toggle_once
  and compact = compact
  and toggle_compact = toggle_compact
  and revision = revision
  and refresh = refresh in
  let appearance =
    S.Appearance.create
      ~dark:
        (match Palette.document_appearance p with
         | Dark -> true
         | Light -> false)
      ~foreground:(Palette.foreground p)
      ~background:(Palette.surface p)
      ()
    |> ok
  in
  let config =
    S.Config.create
      ~appearance
      ~animated:playing
      ~direction:(if reverse then Right_to_left else Left_to_right)
      ~repeat:(if once then Once else Loop)
      ()
    |> ok
  in
  let title =
    V.text
      ~key:(Key.of_string_exn "shimmer-status")
      ~style:
        (style
           [ Foreground (Palette.muted p)
           ; Font_size (Palette.size p 24.)
           ; Line_height (px (Palette.size p 34.))
           ; User_select true
           ; Width (px (if compact then 280. else 480.))
           ; Max_width (Length.percent_exn 100.)
           ])
      (sprintf "Connecting ideas · %d\nAé世界 · é · 👩‍💻" revision)
    |> fun view -> V.with_text_shimmer view (Option.some_if enabled config) |> ok
  in
  V.column
    ~style:(style [ Gap (px 14.); Width (Length.percent_exn 100.) ])
    [ Palette.text p ~muted:true "A quiet sweep of light. The words stay selectable."
    ; V.column
        ~style:
          (style
             [ Padding (px 20.)
             ; Radius 14.
             ; Background (Background.solid (Palette.surface p))
             ; Min_height (px 120.)
             ; Justify_content Center
             ])
        [ title ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button
            p
            (if playing then "Pause text shimmer" else "Start text shimmer")
            toggle_playing
        ; Palette.button p "Refresh shimmer status" (refresh ())
        ; Palette.button
            p
            (if compact then "Shimmer width: compact" else "Shimmer width: wide")
            toggle_compact
        ]
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(if enabled then Checked else Unchecked)
            ~on_toggle:toggle_enabled
            "Text shimmer effect"
        ; V.checkbox
            ~state:(if reverse then Checked else Unchecked)
            ~on_toggle:toggle_reverse
            "Reverse shimmer"
        ; V.checkbox
            ~state:(if once then Checked else Unchecked)
            ~on_toggle:toggle_once
            "One sweep"
        ]
    ; Palette.text
        p
        ~muted:true
        "Reduced motion keeps the text still. Refresh the status to replay a completed \
         sweep."
    ]
;;

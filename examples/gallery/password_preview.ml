open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

let component window palette graph =
  let revealed, toggle_revealed = B.toggle ~default_model:false graph in
  let read_only, toggle_read_only = B.toggle ~default_model:false graph in
  let disabled, toggle_disabled = B.toggle ~default_model:false graph in
  let loading, toggle_loading = B.toggle ~default_model:false graph in
  let menu_enabled, toggle_menu = B.toggle ~default_model:true graph in
  let submitted, set_submitted = B.state false graph in
  let open B.Let_syntax in
  let config =
    let%arr revealed = revealed
    and read_only = read_only
    and disabled = disabled in
    Text_input.Config.create
      ~mode:Single_line
      ~privacy:(Password (if revealed then Revealed else Hidden))
      ~content_hint:Password
      ~label:"Example password"
      ~placeholder:"Try a sample password"
      ~read_only
      ~disabled
      ()
    |> Or_error.ok_exn
  in
  let on_submit =
    let%arr set_submitted = set_submitted in
    fun (_ : Text_input.Submission.t) -> set_submitted true
  in
  let editor =
    Editor.create window ~config ~on_submit ~initial_text:"Sample-λ-42" graph
  in
  let%arr p = palette
  and editor = editor
  and revealed = revealed
  and toggle_revealed = toggle_revealed
  and read_only = read_only
  and toggle_read_only = toggle_read_only
  and disabled = disabled
  and toggle_disabled = toggle_disabled
  and loading = loading
  and toggle_loading = toggle_loading
  and menu_enabled = menu_enabled
  and toggle_menu = toggle_menu
  and submitted = submitted in
  Palette.card
    p
    ~title:"A password, with a little discretion"
    [ Palette.text
        p
        ~muted:true
        "Use sample text here. Reveal your draft, edit it, then hide it again."
    ; Editor.view
        ~style:(Style.create_exn [ Height (Length.px_exn (Palette.size p 42.)) ])
        editor
      |> V.input_frame
           ~config:
             (Input_frame.create
                ~clear_label:"Clear example password"
                ~loading
                ~loading_label:"Checking example password"
                ()
              |> Or_error.ok_exn)
           ~leading:(Palette.text p ~muted:true "Key")
           ~on_reveal:(fun () -> toggle_revealed)
      |> Or_error.ok_exn
      |> V.editor_menu
           ~config:(Editor_menu.create ~enabled:menu_enabled () |> Or_error.ok_exn)
      |> Or_error.ok_exn
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 16.); Wrap Wrap ])
        [ V.checkbox
            ~state:(if revealed then Checked else Unchecked)
            ~on_toggle:toggle_revealed
            "Reveal example password"
        ; V.checkbox
            ~state:(if read_only then Checked else Unchecked)
            ~on_toggle:toggle_read_only
            "Read-only password"
        ; V.checkbox
            ~state:(if disabled then Checked else Unchecked)
            ~on_toggle:toggle_disabled
            "Disable password"
        ; V.checkbox
            ~state:(if loading then Checked else Unchecked)
            ~on_toggle:toggle_loading
            "Show checking indicator"
        ; V.checkbox
            ~state:(if menu_enabled then Checked else Unchecked)
            ~on_toggle:toggle_menu
            "Enable password edit menu"
        ]
    ; Palette.text
        p
        ~muted:true
        "Clear keeps undo history. Checking leaves typing available. Right-click the \
         field, or press Shift-F10 while editing."
    ; Palette.text
        p
        ~muted:true
        (if revealed
         then "Visible text can be copied. Hide it to block copying and cutting."
         else
           "Hidden text cannot be copied or cut. Your editing history stays with the \
            field.")
    ; Palette.text
        p
        (if submitted
         then "Example submitted. Its value is not displayed."
         else "Press Enter to try submitting the example.")
    ]
;;

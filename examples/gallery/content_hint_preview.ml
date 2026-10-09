open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input
module Hint = Text_input.Content_hint

let hint_name hint =
  Hint.sexp_of_t hint |> Sexp.to_string_hum |> String.tr ~target:'_' ~replacement:' '
;;

let describe = function
  | Error error ->
    "Could not check: " ^ (Text_input.Command_error.sexp_of_t error |> Sexp.to_string_hum)
  | Ok (Hint.Status.Inactive None) -> "No hint configured"
  | Ok (Inactive (Some hint)) -> hint_name hint ^ ": inactive for this field"
  | Ok (Exposed hint) -> hint_name hint ^ ": exposed to the native view"
  | Ok (Unavailable (hint, reason)) ->
    let reason =
      match reason with
      | Hint.Status.Unavailability.Backend -> "backend does not support exposure"
      | Mapping -> "no mapping for this hint"
      | Native_view -> "native view cannot expose this hint"
    in
    hint_name hint ^ ": " ^ reason
;;

let component window palette graph =
  let hint, set_hint = B.state (Some Hint.Email_address) graph in
  let read_only, toggle_read_only = B.toggle ~default_model:false graph in
  let last_check, set_last_check = B.state "Not checked yet" graph in
  let open B.Let_syntax in
  let config =
    let%arr content_hint = hint
    and read_only = read_only in
    Text_input.Config.create
      ~mode:Single_line
      ~label:"Contact detail"
      ~placeholder:"Use a sample contact detail"
      ~read_only
      ?content_hint
      ()
    |> Or_error.ok_exn
  in
  let editor = Editor.create window ~config ~initial_text:"hello@example.test" graph in
  let%arr p = palette
  and editor = editor
  and hint = hint
  and set_hint = set_hint
  and read_only = read_only
  and toggle_read_only = toggle_read_only
  and last_check = last_check
  and set_last_check = set_last_check in
  let check =
    let open B.Effect.Let_syntax in
    let%bind result = Editor.content_hint_status editor in
    set_last_check (describe result)
  in
  let focus =
    let open B.Effect.Let_syntax in
    let%bind result = Editor.focus editor in
    match result with
    | Error error -> set_last_check (describe (Error error))
    | Ok _ -> B.Effect.return ()
  in
  Palette.card
    p
    ~title:"A little context for your input"
    [ Palette.text
        p
        ~muted:true
        "Hints describe a field to the system. They do not validate text or guarantee \
         autofill."
    ; Editor.view
        ~style:(Style.create_exn [ Height (Length.px_exn (Palette.size p 42.)) ])
        editor
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 10.); Wrap Wrap ])
        (List.map
           [ "Email", Some Hint.Email_address
           ; "Website", Some Url
           ; "IMEI", Some Cellular_imei
           ; "No hint", None
           ]
           ~f:(fun (label, value) ->
             V.button
               ~config:(Button.Config.create ~focus:Preserve ())
               ~on_click:(set_hint value)
               label))
    ; Palette.text p ("Configured: " ^ Option.value_map hint ~default:"none" ~f:hint_name)
    ; V.switch ~checked:read_only ~on_toggle:toggle_read_only "Read-only contact detail"
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 10.); Wrap Wrap ])
        [ Palette.button p "Focus contact field" focus
        ; V.button
            ~config:(Button.Config.create ~focus:Preserve ())
            ~on_click:check
            "Check native hint"
        ]
    ; Palette.text p ("Last check: " ^ last_check)
    ; Palette.text
        p
        ~muted:true
        "Focus the field, then check. The check preserves focus and reports the hint at \
         query time; check again after changing it. IMEI demonstrates an unavailable \
         mapping on macOS."
    ]
;;

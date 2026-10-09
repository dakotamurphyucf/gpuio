open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

module Mode = struct
  type t =
    | Reference
    | Decimal
    | Free
  [@@deriving equal]

  let format = function
    | Reference ->
      Some (Input_format.Pattern.create "99-99" |> Or_error.ok_exn |> Input_format.pattern)
    | Decimal ->
      Some
        (Input_format.Number.create ~separator:"," ~fraction_digits:2 ()
         |> Or_error.ok_exn
         |> Input_format.number)
    | Free -> None
  ;;

  let sample = function
    | Reference -> "1234"
    | Decimal -> "1234567.50"
    | Free -> "A draft with room to grow"
  ;;
end

let component window palette graph =
  let mode, set_mode = B.state Mode.Reference graph in
  let notice, set_notice = B.state "Ready to edit" graph in
  let open B.Let_syntax in
  let config =
    let%arr mode = mode in
    Text_input.Config.create
      ~mode:Single_line
      ~label:"Formatted draft"
      ~placeholder:"Type a value"
      ?format:(Mode.format mode)
      ()
    |> Or_error.ok_exn
  in
  let editor = Editor.create window ~config ~initial_text:"12-34" graph in
  let%arr p = palette
  and editor = editor
  and mode = mode
  and set_mode = set_mode
  and notice = notice
  and set_notice = set_notice in
  let observe result =
    set_notice
      (match result with
       | Ok _ -> "Sample applied · Undo restores the previous draft"
       | Error error -> Text_input.Command_error.sexp_of_t error |> Sexp.to_string_hum)
  in
  let sample =
    match Mode.format mode with
    | None -> Ok (Mode.sample mode)
    | Some format -> Input_format.format_raw format (Mode.sample mode)
  in
  let load_sample =
    match sample, Editor.snapshot editor with
    | Error error, _ ->
      set_notice (Input_format.Error.sexp_of_t error |> Sexp.to_string_hum)
    | Ok _, None -> set_notice "The field is not ready yet"
    | Ok text, Some expected ->
      let open B.Effect.Let_syntax in
      let%bind result =
        Editor.replace_if_unchanged editor expected ~selection:End ~undo:Record text
      in
      observe result
  in
  let raw_value =
    match Editor.snapshot editor with
    | None -> "Waiting for the field"
    | Some snapshot when Option.is_some (Text_input.Snapshot.composition snapshot) ->
      "Composing…"
    | Some snapshot ->
      let text = Text_input.Snapshot.text snapshot in
      (match Mode.format mode with
       | None -> text
       | Some format ->
         (match Input_format.raw_of_formatted format text with
          | Ok raw -> raw
          | Error _ -> "This retained draft does not fit the selected format yet"))
  in
  Palette.card
    p
    ~title:"A little structure, without losing your draft"
    [ Palette.text
        p
        ~muted:true
        "Switch formats while editing. Your draft stays in place; load a sample when you \
         want a fresh example."
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 10.); Wrap Wrap ])
        (List.map
           [ "Reference · 99-99", Mode.Reference
           ; "Decimal · 1,234.50", Decimal
           ; "Free text", Free
           ]
           ~f:(fun (label, value) ->
             V.button
               ~config:(Button.Config.create ~focus:Preserve ())
               ~on_click:(set_mode value)
               (if Mode.equal mode value then "✓ " ^ label else label)))
    ; Editor.view
        ~style:(Style.create_exn [ Height (Length.px_exn (Palette.size p 42.)) ])
        editor
    ; Palette.text p ("Raw value: " ^ raw_value)
    ; V.button
        ~config:(Button.Config.create ~focus:Preserve ())
        ~on_click:load_sample
        "Load formatted sample"
    ; Palette.text p ~muted:true notice
    ]
;;

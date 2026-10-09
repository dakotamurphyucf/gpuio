open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

type t =
  { handle : Input_validation.t
  ; reference : Input_validation.t
  }

let prepare () =
  let open Or_error.Let_syntax in
  let rule pattern =
    let%bind source = Input_validation.Regex.Source.create pattern in
    match Gpuio_eio.Input_validation.prepare_regex source with
    | Ok regex -> Ok (Input_validation.regex regex)
    | Error error -> Or_error.error_s [%sexp (error : Input_validation.Error.t)]
  in
  let%bind handle = rule "[a-z0-9_-]*" in
  let%map reference = rule "[0-9-]*" in
  { handle; reference }
;;

module Mode = struct
  type t =
    | Handle
    | Reference
    | Free
  [@@deriving equal]
end

let component rules window palette graph =
  let mode, set_mode = B.state Mode.Handle graph in
  let notice, set_notice = B.state "Try typing or pasting into the field" graph in
  let open B.Let_syntax in
  let config =
    let%arr mode = mode in
    let edit_filter, format =
      match mode with
      | Mode.Handle -> Some rules.handle, None
      | Reference ->
        ( Some rules.reference
        , Some
            (Input_format.Pattern.create "99-99"
             |> Or_error.ok_exn
             |> Input_format.pattern) )
      | Free -> None, None
    in
    Text_input.Config.create
      ~mode:Single_line
      ~label:"Filtered draft"
      ~placeholder:"Make it yours"
      ?edit_filter
      ?format
      ()
    |> Or_error.ok_exn
  in
  let editor = Editor.create window ~config ~initial_text:"workspace-17" graph in
  let%arr p = palette
  and mode = mode
  and set_mode = set_mode
  and notice = notice
  and set_notice = set_notice
  and editor = editor in
  let sample, help =
    match mode with
    | Mode.Handle -> "workspace-17", "Lowercase letters, numbers, underscores and hyphens"
    | Reference -> "12-34", "Four digits, with a separator added as you type"
    | Free -> "A little room to explore", "Write freely, with the filter removed"
  in
  let load_sample =
    match Editor.snapshot editor with
    | None -> set_notice "The field is not ready yet"
    | Some expected ->
      let open B.Effect.Let_syntax in
      let%bind result =
        Editor.replace_if_unchanged editor expected ~selection:End ~undo:Record sample
      in
      set_notice
        (match result with
         | Ok _ -> "Sample loaded · Undo brings back your previous draft"
         | Error error -> Text_input.Command_error.sexp_of_t error |> Sexp.to_string_hum)
  in
  Palette.card
    p
    ~title:"Keep typing on track"
    [ Palette.text
        p
        ~muted:true
        "Try letters, digits and punctuation. Switching rules keeps your draft in place; \
         load a sample to start fresh."
    ; V.row
        ~style:(Style.create_exn [ Gap (Length.px_exn 10.); Wrap Wrap ])
        (List.map
           [ "Workspace handle", Mode.Handle; "Reference", Reference; "Free text", Free ]
           ~f:(fun (label, value) ->
             V.button
               ~config:(Button.Config.create ~focus:Preserve ())
               ~on_click:(set_mode value)
               (if Mode.equal mode value then "✓ " ^ label else label)))
    ; Palette.text p ~muted:true help
    ; Editor.view
        ~style:(Style.create_exn [ Height (Length.px_exn (Palette.size p 42.)) ])
        editor
    ; V.button
        ~config:(Button.Config.create ~focus:Preserve ())
        ~on_click:load_sample
        "Load matching sample"
    ; Palette.text p ~muted:true notice
    ; Palette.text
        p
        ~muted:true
        "Edit filters guide typing. Applications still validate complete values before \
         submitting a form."
    ]
;;

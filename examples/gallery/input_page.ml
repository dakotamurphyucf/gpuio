open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module D = Drag_and_drop

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let card_kind = D.Custom_kind.of_string "org.gpuio.gallery.card/v1" |> ok
let greeting = D.Payload.text "A small idea, ready to move. 👋" |> ok
let card = D.Payload.custom ~kind:card_kind ~data:"idea-42" |> ok

module Width_action = struct
  type t =
    | Set of float
    | Adjust of float
end

let payload_label = function
  | D.Payload.Text text -> sprintf "Received text · %d UTF-8 bytes" (String.length text)
  | Files files -> sprintf "Received %d file path(s); no files opened" (List.length files)
  | Custom { kind; data } ->
    sprintf "Received %s · %d bytes" (D.Custom_kind.to_string kind) (String.length data)
;;

let trace_source enabled (event : D.Source_event.t) =
  if not enabled
  then E.Ignore
  else
    E.of_thunk (fun () ->
      let phase =
        match event.phase with
        | Started _ -> "started"
        | Desktop_offered -> "desktop_offered"
        | Desktop_unavailable -> "desktop_unavailable"
        | Ended Internal_drop -> "delivered"
        | Ended Unconfirmed -> "unconfirmed"
        | Ended (Cancelled _) -> "cancelled"
      in
      Eio.traceln
        "GALLERY_TRANSFER_SOURCE gesture=%s phase=%s"
        (Sexp.to_string [%sexp (event.gesture : D.Gesture_id.t)])
        phase)
;;

let trace_target enabled (event : D.Target_event.t) =
  if not enabled
  then E.Ignore
  else (
    match event.phase with
    | Moved -> E.Ignore
    | Entered _ | Left | Dropped _ | Rejected _ ->
      E.of_thunk (fun () ->
        let phase =
          match event.phase with
          | Entered _ -> "entered"
          | Left -> "left"
          | Dropped _ -> "dropped"
          | Rejected _ -> "rejected"
          | Moved -> assert false
        in
        Eio.traceln
          "GALLERY_TRANSFER_TARGET gesture=%s phase=%s"
          (Sexp.to_string [%sexp (event.gesture : D.Gesture_id.t)])
          phase;
        match event.phase with
        | Entered offer ->
          Eio.traceln
            "GALLERY_TRANSFER_ORIGIN %s"
            (Sexp.to_string [%sexp (offer.origin : D.Origin.t)])
        | Dropped (Files files) ->
          List.iteri files ~f:(fun index file ->
            let path_hex =
              File_path.to_string file.path
              |> String.to_list
              |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
              |> String.concat
            in
            Eio.traceln
              "GALLERY_TRANSFER_FILE index=%d path_hex=%s directory=%s"
              index
              path_hex
              (Sexp.to_string [%sexp (file.is_directory : bool option)]))
        | Moved | Left | Dropped _ | Rejected _ -> ()))
;;

let component palette graph =
  let trace = Array.exists (Sys.get_argv ()) ~f:(String.equal "--trace-input") in
  let width, change_width =
    B.state_machine0
      ~default_model:180.
      ~apply_action:(fun _ width action ->
        let candidate =
          match action with
          | Width_action.Set value -> value
          | Adjust amount -> width +. amount
        in
        Float.clamp_exn candidate ~min:80. ~max:360.)
      graph
  in
  let pointer_disabled, toggle_pointer = B.toggle ~default_model:false graph in
  let pointer_notice, set_pointer_notice = B.state "Ready to drag" graph in
  let drag_disabled, toggle_drag = B.toggle ~default_model:false graph in
  let custom, toggle_custom = B.toggle ~default_model:false graph in
  let accepts_cards, toggle_accepts = B.toggle ~default_model:true graph in
  let source_notice, set_source_notice = B.state "No active transfer" graph in
  let hovered, set_hovered = B.state false graph in
  let receipt, set_receipt = B.state "Nothing received yet" graph in
  let count, count_drop =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ count () -> if count = Int.max_value then count else count + 1)
      graph
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_pointer_notice = set_pointer_notice
       and set_source_notice = set_source_notice
       and set_hovered = set_hovered in
       E.Many
         [ set_pointer_notice "Ready to drag"
         ; set_source_notice "No active transfer"
         ; set_hovered false
         ])
    graph;
  let%arr p = palette
  and width = width
  and change_width = change_width
  and pointer_disabled = pointer_disabled
  and toggle_pointer = toggle_pointer
  and pointer_notice = pointer_notice
  and set_pointer_notice = set_pointer_notice
  and drag_disabled = drag_disabled
  and toggle_drag = toggle_drag
  and custom = custom
  and toggle_custom = toggle_custom
  and accepts_cards = accepts_cards
  and toggle_accepts = toggle_accepts
  and source_notice = source_notice
  and set_source_notice = set_source_notice
  and hovered = hovered
  and set_hovered = set_hovered
  and receipt = receipt
  and set_receipt = set_receipt
  and count = count
  and count_drop = count_drop in
  let controls children = V.row ~style:(style [ Gap (px 8.); Wrap Wrap ]) children in
  let region ~accent =
    style
      [ Width (px 250.)
      ; Height (px 92.)
      ; Shrink 0.
      ; Padding (px 16.)
      ; Radius 12.
      ; Background
          (Background.solid (if accent then Palette.border p else Palette.background p))
      ; Foreground (Palette.foreground p)
      ; Border_width 1.
      ; Border_color (if accent then Palette.accent p else Palette.border p)
      ]
  in
  let payload = if custom then card else greeting in
  let receive payload = E.Many [ set_receipt (payload_label payload); count_drop () ] in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"A gesture with a clear beginning and end"
        [ Palette.text
            p
            ~muted:true
            "Drag along the track, or use the size buttons. Escape cancels a captured \
             gesture."
        ; controls
            [ Palette.button p "Narrow panel" (change_width (Adjust (-20.)))
            ; Palette.button p "Widen panel" (change_width (Adjust 20.))
            ; Palette.button p "Reset panel" (change_width (Set 180.))
            ; Palette.button
                p
                (if pointer_disabled
                 then "Enable pointer input"
                 else "Disable pointer input")
                toggle_pointer
            ]
        ; V.pointer_area
            ~key:(Key.of_string_exn "gallery-pointer-track")
            ~style:
              (style
                 [ Width (px 360.)
                 ; Height (px 48.)
                 ; Radius 10.
                 ; Padding (px 12.)
                 ; Background (Background.solid (Palette.border p))
                 ; Foreground (Palette.foreground p)
                 ])
            ~config:
              (Pointer.Config.create
                 ~label:"Panel drag track"
                 ~disabled:pointer_disabled
                 ()
               |> ok)
            ~on_event:(fun event ->
              match event.Pointer.Event.phase with
              | Started | Moved | Released ->
                E.Many
                  [ change_width (Set event.local_position.x)
                  ; set_pointer_notice
                      (Sexp.to_string_hum [%sexp (event.phase : Pointer.Phase.t)])
                  ]
              | Cancelled _ ->
                set_pointer_notice
                  (Sexp.to_string_hum [%sexp (event.phase : Pointer.Phase.t)]))
            [ Palette.text p "Drag to size your panel" ]
        ; (V.column
             ~style:
               (style
                  [ Width (px width)
                  ; Height (px 36.)
                  ; Radius 8.
                  ; Background (Background.solid (Palette.accent p))
                  ])
             []
           |> fun view ->
           V.with_accessibility
             view
             (Accessibility.create ~role:Group ~label:"Sized panel" () |> ok)
           |> ok)
        ; Palette.text p (sprintf "Panel width: %.0f logical pixels" width)
        ; Palette.text p ("Pointer: " ^ pointer_notice)
        ]
    ; Palette.card
        p
        ~title:"A small idea, ready to move"
        [ controls
            [ Palette.button
                p
                (if custom then "Use text payload" else "Use card payload")
                toggle_custom
            ; Palette.button
                p
                (if accepts_cards then "Reject cards" else "Accept cards")
                toggle_accepts
            ; Palette.button
                p
                (if drag_disabled then "Enable transfers" else "Disable transfers")
                toggle_drag
            ]
        ; V.row
            ~style:(style [ Gap (px 16.); Wrap Wrap ])
            [ V.drag_source
                ~key:(Key.of_string_exn "gallery-transfer-source")
                ~style:(region ~accent:true)
                ~config:
                  (D.Source.create
                     ~label:"Idea transfer source"
                     ~payload
                     ~disabled:drag_disabled
                     ()
                   |> ok)
                ~on_event:(fun event ->
                  E.Many
                    [ trace_source trace event
                    ; set_source_notice
                        (match event.D.Source_event.phase with
                         | Started _ -> "Transfer started"
                         | Desktop_offered -> "Offered to the desktop"
                         | Desktop_unavailable -> "Desktop offering unavailable"
                         | Ended Internal_drop -> "Transfer delivered"
                         | Ended Unconfirmed -> "Transfer ended without an accepted drop"
                         | Ended (Cancelled reason) ->
                           "Transfer cancelled: "
                           ^ Sexp.to_string_hum [%sexp (reason : D.Cancel_reason.t)])
                    ])
                [ Palette.text
                    p
                    (if custom then "Drag this idea card" else "Drag this greeting")
                ]
            ; V.drop_target
                ~key:(Key.of_string_exn "gallery-transfer-target")
                ~style:(region ~accent:hovered)
                ~config:
                  (D.Target.create
                     ~label:"Idea transfer inbox"
                     ~accept:
                       ([ D.Format.Text; Files ]
                        @ if accepts_cards then [ Custom card_kind ] else [])
                     ~disabled:drag_disabled
                     ()
                   |> ok)
                ~on_event:(fun event ->
                  E.Many
                    [ trace_target trace event
                    ; (match event.D.Target_event.phase with
                       | Entered _ -> set_hovered true
                       | Moved -> E.Ignore
                       | Left -> set_hovered false
                       | Dropped payload -> E.Many [ set_hovered false; receive payload ]
                       | Rejected reason ->
                         E.Many
                           [ set_hovered false
                           ; set_receipt
                               ("Rejected: "
                                ^ Sexp.to_string_hum [%sexp (reason : D.Rejection.t)])
                           ])
                    ])
                [ Palette.text
                    p
                    (if hovered then "Release to receive" else "Drop text or files here")
                ]
            ]
        ; Palette.text p source_notice
        ; Palette.text p receipt
        ; Palette.text p (sprintf "Received items: %d" count)
        ; Palette.button
            p
            ~disabled:(drag_disabled || (custom && not accepts_cards))
            "Receive with keyboard or click"
            (receive payload)
        ; Palette.text
            p
            ~muted:true
            "The inbox accepts text and file paths; cards are optional. Native rules \
             decide acceptance. Files are counted, never opened."
        ]
    ]
;;

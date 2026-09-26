open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module C = Gpuio.Color_input
module V = Gpuio.Color_value
module P = Gpuio_eio.Color_picker
module Policy = Gpuio.Color_picker
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let color hex = V.Value.Color (V.Rgba.of_hex hex |> ok)
let initial = color "#7C6FF080"
let draft_color = color "#54C6A2"
let replacement = color "#EF6B95"

let overlay =
  Gpuio.Overlay.Config.create
    ~label:"Choose accent color"
    ~width:380.
    ~dismiss_on_outside_pointer:true
    ()
  |> ok
;;

let frame window =
  E.Expert.of_fun ~f:(fun ~callback ->
    App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
      E.of_thunk (fun () -> callback ()))
    |> ok)
;;

let settle window = E.bind (frame window) ~f:(fun () -> frame window)

let native = function
  | Ok snapshot -> E.return snapshot
  | Error error -> E.of_thunk (fun () -> raise_s [%sexp (error : C.Command_error.t)])
;;

let expect = function
  | Ok selection -> E.return selection
  | Error error -> E.of_thunk (fun () -> raise_s [%sexp (error : Policy.Error.t)])
;;

let error result expected =
  E.of_thunk (fun () ->
    if not (Result.equal V.Value.equal Policy.Error.equal result (Error expected))
    then
      raise_s
        [%message
          "Unexpected picker result"
            (result : (V.Value.t, Policy.Error.t) Result.t)
            (expected : Policy.Error.t)])
;;

let selection actual expected =
  E.of_thunk (fun () -> assert (V.Value.equal actual expected))
;;

let cancel_during_confirmation picker =
  E.Expert.of_fun ~f:(fun ~callback ->
    E.Expert.eval (P.confirm picker) ~f:callback;
    E.Expert.eval (P.cancel picker) ~f:(fun () -> ()))
;;

type flags =
  { disabled : bool
  ; read_only : bool
  ; restricted : bool
  }
[@@deriving equal]

let normal = { disabled = false; read_only = false; restricted = false }

let exercise
      window
      ~picker
      ~latest
      ~application_value
      ~confirmed
      ~set_value
      ~set_flags
      ~set_placed
      ~set_active
      ~completed
  =
  let current () = E.of_thunk (fun () -> Option.value_exn !latest) in
  let reopen () =
    let open E.Let_syntax in
    let%bind t = current () in
    let%bind () = P.open_popup t in
    let%bind () = settle window in
    current ()
  in
  let replace t selection =
    E.bind (P.command t (Set { value = selection; if_revision = None })) ~f:native
  in
  let open E.Let_syntax in
  let%bind closed = P.confirm picker in
  let%bind () = error closed Not_open in
  let%bind first = reopen () in
  let%bind () = E.of_thunk (fun () -> assert (P.is_open first)) in
  let%bind () =
    selection (C.Snapshot.value (P.draft first |> Option.value_exn)) initial
  in
  let%bind _ = replace first replacement in
  let%bind applied = P.confirm first >>= expect in
  let%bind () = selection applied replacement in
  let%bind () = settle window in
  let%bind now = current () in
  let%bind () =
    E.of_thunk (fun () ->
      assert (not (P.is_open now));
      assert (List.length !confirmed = 1))
  in
  let%bind () = selection !application_value replacement in
  let%bind second = reopen () in
  let%bind _ = replace second draft_color in
  let%bind () = P.cancel second in
  let%bind () = settle window in
  let%bind third = reopen () in
  let%bind () =
    selection (C.Snapshot.value (P.draft third |> Option.value_exn)) replacement
  in
  let%bind () = P.cancel second in
  let%bind () = settle window in
  let%bind still_open = current () in
  let%bind () = E.of_thunk (fun () -> assert (P.is_open still_open)) in
  let%bind stale = P.command second Read_snapshot in
  let%bind () =
    E.of_thunk (fun () ->
      assert (
        Result.equal
          C.Snapshot.equal
          C.Command_error.equal
          stale
          (Error Stale_color_input)))
  in
  let%bind _ = replace third draft_color in
  let%bind () = set_placed false in
  let%bind () = settle window in
  let%bind () = set_placed true in
  let%bind () = settle window in
  let%bind remounted = current () in
  let%bind () =
    selection (C.Snapshot.value (P.draft remounted |> Option.value_exn)) replacement
  in
  let%bind obsolete = P.confirm third in
  let%bind () = error obsolete (Native Stale_color_input) in
  let%bind cancelled = cancel_during_confirmation remounted in
  let%bind () = error cancelled Stale_session in
  let%bind () = settle window in
  let%bind closed = current () in
  let%bind () =
    E.of_thunk (fun () ->
      assert (not (P.is_open closed));
      assert (List.length !confirmed = 1))
  in
  let%bind _ = reopen () in
  let%bind () = set_active false in
  let%bind () = settle window in
  let%bind () = E.of_thunk (fun () -> assert (Option.is_none !latest)) in
  let%bind () = set_active true in
  let%bind () = settle window in
  let%bind reactivated = current () in
  let%bind () =
    E.of_thunk (fun () ->
      assert (not (P.is_open reactivated));
      assert (Option.is_none (P.draft reactivated)))
  in
  let%bind _ = reopen () in
  let%bind () = set_flags { normal with read_only = true } in
  let%bind () = settle window in
  let%bind readonly = current () in
  let%bind rejected = P.confirm readonly in
  let%bind () = error rejected Read_only in
  let%bind () = set_flags normal in
  let%bind () = set_value initial in
  let%bind () = settle window in
  let%bind closed = current () in
  let%bind () = E.of_thunk (fun () -> assert (not (P.is_open closed))) in
  let%bind stale = P.confirm third in
  let%bind () =
    E.of_thunk (fun () ->
      assert (Result.is_error stale);
      assert (List.length !confirmed = 1))
  in
  let%bind fresh = reopen () in
  let%bind () = set_flags { normal with restricted = true } in
  let%bind () = settle window in
  let%bind disallowed = P.confirm fresh in
  let%bind () = error disallowed Disallowed_value in
  let%bind () = P.cancel fresh in
  let%bind () = settle window in
  let%bind historical = reopen () in
  let%bind () =
    selection (C.Snapshot.value (P.draft historical |> Option.value_exn)) V.Value.Empty
  in
  let%bind () = selection !application_value initial in
  let%bind () = P.cancel historical in
  let%bind () = settle window in
  let%bind () = selection !application_value initial in
  let%bind () = set_flags normal in
  let%bind _ = reopen () in
  let%bind () = set_flags { normal with disabled = true } in
  let%bind () = settle window in
  let%bind disabled = current () in
  let%bind () = E.of_thunk (fun () -> assert (not (P.is_open disabled))) in
  let%bind () = P.open_popup disabled in
  let%bind () = settle window in
  let%bind disabled = current () in
  let%bind () = E.of_thunk (fun () -> assert (not (P.is_open disabled))) in
  let%bind () = set_flags normal in
  let%bind fresh = reopen () in
  let%bind _ = replace fresh V.Value.Empty in
  let%bind applied = P.confirm fresh >>= expect in
  let%bind () = selection applied V.Value.Empty in
  let%bind () = settle window in
  let%bind () = selection !application_value V.Value.Empty in
  let%bind final = reopen () in
  let%bind () = E.of_thunk (fun () -> App.Window.close window) in
  let%bind closed = P.confirm final in
  let%bind () = error closed (Native Closed) in
  E.of_thunk (fun () -> completed := true)
;;

let format = function
  | V.Value.Empty -> "Choose color"
  | Color color -> V.Rgba.to_hex color
;;

let error_message = function
  | Policy.Error.Composing -> "Finish composing before applying."
  | Invalid_draft -> "Enter a valid color before applying."
  | Drag_in_progress -> "Finish the channel drag before applying."
  | Disallowed_value -> "This color does not fit the current policy."
  | Read_only -> "This color is read-only."
  | Disabled -> "Color selection is disabled."
  | Not_ready -> "The color editor is opening."
  | Stale_draft -> "The draft changed. Review it and apply again."
  | Stale_session | Not_open -> "Open the picker to choose a color."
  | Limit_exceeded | Native _ -> "The color editor is unavailable. Close and reopen it."
;;

let component ~self_test ~completed window graph =
  let open B.Let_syntax in
  let value, set_value = B.state initial graph in
  let flags, set_flags = B.state normal graph in
  let placed, set_placed = B.state true graph in
  let active, set_active = B.state true graph in
  let dialog_open, set_dialog = B.state false graph in
  let right_edge, set_right_edge = B.state false graph in
  let confirmed = ref [] in
  let application_value = ref initial in
  let config =
    let%arr flags = flags in
    C.Config.create
      ~labels:(C.Labels.english ~control:"Accent color" |> ok)
      ~alpha_policy:(if flags.restricted then Opaque_only else Allow_alpha)
      ~allow_empty:true
      ~palette:
        (List.map
           [ "Iris", "#7C6FF0"; "Rose", "#EF6B95"; "Mint", "#54C6A2" ]
           ~f:(fun (label, hex) ->
             C.Palette_entry.create ~label ~color:(V.Rgba.of_hex hex |> ok) |> ok))
      ~disabled:flags.disabled
      ~read_only:flags.read_only
      ()
    |> ok
  in
  let on_change =
    let%arr set_value = set_value in
    fun value ->
      E.Many [ E.of_thunk (fun () -> confirmed := value :: !confirmed); set_value value ]
  in
  let entries =
    B.map active ~f:(fun active ->
      if active then Int.Map.singleton 0 () else Int.Map.empty)
  in
  let controllers =
    B.assoc
      (module Int)
      entries
      ~f:(fun _ _ graph -> P.create window ~config ~value ~on_change graph)
      graph
  in
  let picker = B.map controllers ~f:(fun controllers -> Map.find controllers 0) in
  let latest = ref None in
  let tracked =
    B.map2 picker value ~f:(fun picker value ->
      ( Option.map picker ~f:(fun picker ->
          P.is_open picker, P.draft picker, P.error picker)
      , value ))
  in
  B.Edge.on_change
    tracked
    ~equal:
      [%equal: (bool * C.Snapshot.t option * Policy.Error.t option) option * V.Value.t]
    ~callback:
      (let%arr picker = picker
       and value = value in
       fun _ ->
         E.of_thunk (fun () ->
           latest := picker;
           application_value := value))
    graph;
  let started = ref false in
  B.Edge.lifecycle
    ~on_activate:
      (let%arr picker = picker
       and set_value = set_value
       and set_flags = set_flags
       and set_placed = set_placed
       and set_active = set_active in
       let open E.Let_syntax in
       let%bind start =
         E.of_thunk (fun () ->
           if self_test && not !started
           then (
             started := true;
             latest := picker;
             true)
           else false)
       in
       if start
       then
         exercise
           window
           ~picker:(Option.value_exn picker)
           ~latest
           ~application_value
           ~confirmed
           ~set_value
           ~set_flags
           ~set_placed
           ~set_active
           ~completed
       else E.Ignore)
    graph;
  let%arr picker = picker
  and value = value
  and set_value = set_value
  and flags = flags
  and set_flags = set_flags
  and dialog_open = dialog_open
  and set_dialog = set_dialog
  and right_edge = right_edge
  and set_right_edge = set_right_edge
  and placed = placed in
  let picker_view =
    match picker with
    | Some picker when placed -> P.view ~overlay ~label:(format value) picker
    | Some _ | None -> View.column []
  in
  let cancel_picker = Option.value_map picker ~default:E.Ignore ~f:P.cancel in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Width (Gpuio.Length.percent_exn 100.)
         ; Height (Gpuio.Length.percent_exn 100.)
         ; Background (Gpuio.Background.solid (Gpuio.Color.rgb_exn 0x0d1420))
         ; Foreground (Gpuio.Color.rgb_exn 0xe6edf3)
         ; Padding (Gpuio.Length.px_exn 28.)
         ; Gap (Gpuio.Length.px_exn 18.)
         ])
    [ View.text "Make it yours"
    ; View.text "Choose an accent, then Apply. Escape or Cancel preserves your color."
    ; View.row
        ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
        [ View.button ~on_click:(set_value initial) "Reset color"
        ; View.button
            ~on_click:(set_flags { flags with read_only = not flags.read_only })
            (if flags.read_only then "Allow selection" else "Read-only")
        ; View.button
            ~on_click:(set_flags { flags with disabled = not flags.disabled })
            (if flags.disabled then "Enable" else "Disable")
        ; View.button ~on_click:(E.of_thunk (fun () -> App.Window.close window)) "Close"
        ]
    ; View.row
        ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
        [ View.button
            ~on_click:(set_flags { flags with restricted = not flags.restricted })
            (if flags.restricted then "Allow transparency" else "Opaque only")
        ; View.button ~on_click:(E.Many [ cancel_picker; set_dialog true ]) "Open dialog"
        ; View.button
            ~on_click:(E.Many [ cancel_picker; set_right_edge (not right_edge) ])
            (if right_edge then "Left edge" else "Right edge")
        ]
    ; (if dialog_open
       then View.column []
       else
         View.row
           ~style:
             (Gpuio.Style.create_exn
                [ Justify_content (if right_edge then End else Start) ])
           [ picker_view ])
    ; View.dialog
        ~key:(Gpuio.Key.of_string_exn "picker-dialog")
        ~config:
          (Gpuio.Overlay.Config.create ~label:"Workspace appearance" ~width:440. () |> ok)
        ~on_dismiss:(fun _ -> E.Many [ cancel_picker; set_dialog false ])
        (if dialog_open
         then
           Some
             (View.column
                ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 12.) ])
                [ View.text "Workspace appearance"
                ; picker_view
                ; View.button
                    ~on_click:(E.Many [ cancel_picker; set_dialog false ])
                    "Done"
                ])
         else None)
    ; View.text ("Confirmed: " ^ format value)
    ; View.text
        (Option.value_map (Option.bind picker ~f:P.error) ~default:"" ~f:error_message)
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    ignore
      (App.open_window
         app
         ~title:"GPUIO color picker"
         ~width:760.
         ~height:600.
         (component ~self_test ~completed)
       |> ok
       : App.Window.t));
  if self_test
  then (
    assert !completed;
    Eio_main.run (fun env ->
      Eio.Flow.copy_string
        "GPUIO_COLOR_PICKER_PUBLIC_OK: apply/cancel, policy, historical fallback, \
         external reset, stale sessions, remount and close\n"
        (Eio.Stdenv.stdout env)))
;;

open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module C = Gpuio.Calendar
module P = Gpuio_eio.Date_picker
module Policy = Gpuio.Date_picker
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let day = Date.of_string
let month = C.Month.of_date (day "2024-02-01") |> ok

let range first last =
  C.Range.create ~first:(day first) ~last:(day last) |> ok |> C.Selection.range
;;

let initial = range "2024-02-27" "2024-02-29"
let single_initial = C.Selection.single (day "2024-02-29") |> ok
let partial = C.Selection.range_start (day "2024-02-20") |> ok
let replacement = range "2024-02-20" "2024-02-22"

let overlay =
  Gpuio.Overlay.Config.create
    ~label:"Choose travel dates"
    ~width:340.
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
    if not (Result.equal C.Selection.equal Policy.Error.equal result (Error expected))
    then
      raise_s
        [%message
          "Unexpected picker result"
            (result : (C.Selection.t, Policy.Error.t) Result.t)
            (expected : Policy.Error.t)])
;;

let selection actual expected =
  E.of_thunk (fun () -> assert (C.Selection.equal actual expected))
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
  ; single : bool
  }
[@@deriving equal]

let normal = { disabled = false; read_only = false; restricted = false; single = false }

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
    E.bind (P.command t (Replace { selection; if_revision = None })) ~f:native
  in
  let open E.Let_syntax in
  let%bind closed = P.confirm picker in
  let%bind () = error closed Not_open in
  let%bind first = reopen () in
  let%bind () = E.of_thunk (fun () -> assert (P.is_open first)) in
  let%bind () =
    selection (C.Snapshot.selection (P.draft first |> Option.value_exn)) initial
  in
  let%bind _ = replace first partial in
  let%bind rejected = P.confirm first in
  let%bind () = error rejected Incomplete_range in
  let%bind () = E.of_thunk (fun () -> assert (List.is_empty !confirmed)) in
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
  let%bind _ = replace second partial in
  let%bind () = P.cancel second in
  let%bind () = settle window in
  let%bind third = reopen () in
  let%bind () =
    selection (C.Snapshot.selection (P.draft third |> Option.value_exn)) replacement
  in
  let%bind () = P.cancel second in
  let%bind () = settle window in
  let%bind still_open = current () in
  let%bind () = E.of_thunk (fun () -> assert (P.is_open still_open)) in
  let%bind stale = P.command second Read_snapshot in
  let%bind () =
    E.of_thunk (fun () ->
      assert (
        Result.equal C.Snapshot.equal C.Command_error.equal stale (Error Stale_input)))
  in
  let%bind _ = replace third partial in
  let%bind () = set_placed false in
  let%bind () = settle window in
  let%bind () = set_placed true in
  let%bind () = settle window in
  let%bind remounted = current () in
  let%bind () =
    selection (C.Snapshot.selection (P.draft remounted |> Option.value_exn)) replacement
  in
  let%bind obsolete = P.confirm third in
  let%bind () = error obsolete (Native Stale_input) in
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
  let%bind () = error disallowed Disallowed_selection in
  let%bind () = P.cancel fresh in
  let%bind () = settle window in
  let%bind historical = reopen () in
  let%bind () =
    selection
      (C.Snapshot.selection (P.draft historical |> Option.value_exn))
      C.Selection.empty
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
  let%bind obsolete_range = reopen () in
  let%bind () = set_flags { normal with single = true } in
  let%bind () = set_value single_initial in
  let%bind () = settle window in
  let%bind changed_mode = current () in
  let%bind () = E.of_thunk (fun () -> assert (not (P.is_open changed_mode))) in
  let%bind stale = P.confirm obsolete_range in
  let%bind () = error stale Stale_session in
  let%bind single = reopen () in
  let%bind () =
    selection (C.Snapshot.selection (P.draft single |> Option.value_exn)) single_initial
  in
  let single_replacement = C.Selection.single (day "2024-03-01") |> ok in
  let%bind _ = replace single single_replacement in
  let%bind applied = P.confirm single >>= expect in
  let%bind () = selection applied single_replacement in
  let%bind () = settle window in
  let%bind () = selection !application_value single_replacement in
  let%bind single = reopen () in
  let%bind _ = replace single C.Selection.empty in
  let%bind () = P.cancel single in
  let%bind () = settle window in
  let%bind () = selection !application_value single_replacement in
  let%bind final = reopen () in
  let%bind () = E.of_thunk (fun () -> App.Window.close window) in
  let%bind closed = P.confirm final in
  let%bind () = error closed (Native Closed) in
  E.of_thunk (fun () -> completed := true)
;;

let format selection =
  let date value = C.Format.format Iso value |> ok in
  match selection with
  | C.Selection.Empty -> "Choose dates"
  | Single value -> date value
  | Range_start value -> date value ^ " – …"
  | Range range -> date (C.Range.first range) ^ " – " ^ date (C.Range.last range)
;;

let error_message = function
  | Policy.Error.Incomplete_range -> "Choose an end date before applying."
  | Disallowed_selection -> "These dates are no longer available. Choose another range."
  | Read_only -> "These dates are read-only."
  | Disabled -> "Date selection is currently disabled."
  | Not_ready -> "The calendar is opening. Please try again."
  | Stale_draft -> "The selection changed. Review it and apply again."
  | Stale_session | Not_open -> "The picker has closed. Open it to choose dates."
  | Wrong_mode | Limit_exceeded | Native _ ->
    "The calendar is unavailable. Close and reopen the picker."
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
    let constraints =
      if flags.restricted
      then C.Constraints.create ~disabled_dates:[ day "2024-02-29" ] () |> ok
      else C.Constraints.unrestricted
    in
    C.Config.create
      ~mode:(if flags.single then Single else Range)
      ~label:(if flags.single then "Appointment date" else "Travel date range")
      ~constraints
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
      ~f:(fun _ _ graph ->
        P.create window ~config ~value ~initial_month:month ~on_change graph)
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
      [%equal:
        (bool * C.Snapshot.t option * Policy.Error.t option) option * C.Selection.t]
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
    [ View.text "Plan a little time away"
    ; View.text "Pick dates, then Apply. Escape or Cancel leaves your dates unchanged."
    ; View.row
        ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
        [ View.button
            ~on_click:(set_value (if flags.single then single_initial else initial))
            "Reset dates"
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
            ~on_click:
              (E.Many
                 [ set_flags { flags with single = not flags.single }
                 ; set_value (if flags.single then initial else single_initial)
                 ])
            (if flags.single then "Date range" else "Single date")
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
          (Gpuio.Overlay.Config.create ~label:"Schedule appointment" ~width:420. () |> ok)
        ~on_dismiss:(fun _ -> E.Many [ cancel_picker; set_dialog false ])
        (if dialog_open
         then
           Some
             (View.column
                ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 12.) ])
                [ View.text "Schedule appointment"
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
         ~title:"GPUIO date picker"
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
        "GPUIO_DATE_PICKER_PUBLIC_OK: single/range, mode changes, partial/complete, \
         apply/cancel, policy, external reset, stale sessions and close\n"
        (Eio.Stdenv.stdout env)))
;;

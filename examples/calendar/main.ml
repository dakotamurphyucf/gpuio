open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Calendar = Gpuio.Calendar
module C = Gpuio_eio.Calendar
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let day = Date.of_string
let month text = Calendar.Month.of_date (day text) |> ok
let single text = Calendar.Selection.single (day text) |> ok
let partial text = Calendar.Selection.range_start (day text) |> ok

let range first last =
  Calendar.Range.create ~first:(day first) ~last:(day last)
  |> ok
  |> Calendar.Selection.range
;;

let initial = single "2024-02-29"
let initial_month = month "2024-02-01"

let constraints =
  Calendar.Constraints.create ~disabled_dates:[ day "2024-03-06" ] () |> ok
;;

let expect = function
  | Ok snapshot -> E.return snapshot
  | Error error ->
    E.of_thunk (fun () -> raise_s [%sexp (error : Calendar.Command_error.t)])
;;

let error result expected =
  E.of_thunk (fun () ->
    if
      not
        (Result.equal
           Calendar.Snapshot.equal
           Calendar.Command_error.equal
           result
           (Error expected))
    then
      raise_s
        [%message
          "Unexpected calendar response"
            (result : (Calendar.Snapshot.t, Calendar.Command_error.t) Result.t)
            (expected : Calendar.Command_error.t)])
;;

let check snapshot selection =
  E.of_thunk (fun () ->
    assert (Calendar.Selection.equal (Calendar.Snapshot.selection snapshot) selection))
;;

let frame window =
  E.Expert.of_fun ~f:(fun ~callback ->
    App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
      E.of_thunk (fun () -> callback ()))
    |> ok)
;;

let settle window = E.bind (frame window) ~f:(fun () -> frame window)

(* Admit every request in one UI turn, before asynchronous replies drain. *)
let concurrent effects =
  E.Expert.of_fun ~f:(fun ~callback ->
    match effects with
    | [] -> callback []
    | _ ->
      let remaining = ref (List.length effects) in
      let results = Array.create ~len:!remaining None in
      List.iteri effects ~f:(fun index pending ->
        E.Expert.eval pending ~f:(fun result ->
          results.(index) <- Some result;
          Int.decr remaining;
          if !remaining = 0
          then
            callback
              (Array.to_list results
               |> List.map ~f:(fun result -> Option.value_exn result)))))
;;

type policy =
  { disabled : bool
  ; read_only : bool
  ; hidden : bool
  }
[@@deriving equal]

let normal = { disabled = false; read_only = false; hidden = false }

let run_self_test
      window
      ~code
      ~span
      ~unplaced
      ~latest
      ~set_policy
      ~set_shown
      ~received
      ~completed
  =
  let open E.Let_syntax in
  let%bind () = settle window in
  let%bind unmounted = C.read_snapshot unplaced in
  let%bind () = error unmounted Not_mounted in
  let%bind first = C.read_snapshot code >>= expect in
  let%bind () = check first initial in
  let%bind again = C.read_snapshot code >>= expect in
  let%bind () = E.of_thunk (fun () -> assert (Calendar.Snapshot.equal first again)) in
  let%bind reads = concurrent (List.init 65 ~f:(fun _ -> C.read_snapshot code)) in
  let%bind () =
    E.of_thunk (fun () ->
      assert (List.count reads ~f:Result.is_ok = 64);
      assert (
        List.count reads ~f:(function
          | Error Calendar.Command_error.Busy -> true
          | _ -> false)
        = 1))
  in
  let%bind focused = C.focus code >>= expect in
  let%bind () = E.of_thunk (fun () -> assert (Calendar.Snapshot.focused focused)) in
  let%bind changed =
    C.replace_if_unchanged code focused (single "2024-03-04") >>= expect
  in
  let%bind () = check changed (single "2024-03-04") in
  let%bind stale = C.replace_if_unchanged code focused initial in
  let%bind () = error stale Stale_revision in
  let%bind wrong = C.replace code (partial "2024-03-01") in
  let%bind () = error wrong Wrong_mode in
  let%bind invalid = C.replace code (single "2024-03-06") in
  let%bind () = error invalid Disabled_date in
  let%bind invalid = C.move_months code ~months:Int.max_value in
  let%bind () = error invalid Invalid_value in
  let%bind moved = C.show_month code (month "2025-01-01") >>= expect in
  let%bind () = check moved (single "2024-03-04") in
  let%bind moved = C.move_months code ~months:1 >>= expect in
  let%bind () =
    E.of_thunk (fun () ->
      assert (Calendar.Month.equal (Calendar.Snapshot.month moved) (month "2025-02-01")))
  in
  let%bind discovered = C.focus_date code (day "2024-03-06") >>= expect in
  let%bind () =
    E.of_thunk (fun () ->
      assert (Date.equal (Calendar.Snapshot.focused_date discovered) (day "2024-03-06"));
      assert (Calendar.Snapshot.focused discovered))
  in
  let%bind () = check discovered (single "2024-03-04") in
  let%bind years = C.set_presentation code Years >>= expect in
  let%bind () =
    E.of_thunk (fun () ->
      assert (Calendar.Presentation.equal (Calendar.Snapshot.presentation years) Years))
  in
  let%bind _ = C.set_presentation code Days >>= expect in
  let%bind _ = C.replace span (partial "2024-03-04") >>= expect in
  let%bind invalid = C.replace span (range "2024-03-04" "2024-03-08") in
  let%bind () = error invalid Disabled_interior in
  let%bind selected = C.replace span (range "2024-03-04" "2024-03-05") >>= expect in
  let%bind () = check selected (range "2024-03-04" "2024-03-05") in
  let%bind () = set_policy { normal with disabled = true } in
  let%bind () = settle window in
  let%bind denied = C.focus code in
  let%bind () = error denied Focus_blocked in
  let%bind _ = C.replace code initial >>= expect in
  let%bind () = set_policy { normal with read_only = true } in
  let%bind () = settle window in
  let%bind _ = C.focus code >>= expect in
  let%bind cleared = C.clear code () >>= expect in
  let%bind () = check cleared Calendar.Selection.empty in
  let%bind () = set_policy { normal with hidden = true } in
  let%bind () = settle window in
  let%bind denied = C.focus_date code (day "2028-01-01") in
  let%bind () = error denied Focus_blocked in
  let%bind hidden = C.read_snapshot code >>= expect in
  let%bind () = E.of_thunk (fun () -> assert (not (Calendar.Snapshot.focused hidden))) in
  let%bind changed = C.replace code (single "2024-04-01") >>= expect in
  let%bind () = check changed (single "2024-04-01") in
  let%bind () = set_policy normal in
  let%bind () = set_shown false in
  let%bind () = settle window in
  let%bind stale = C.read_snapshot code in
  let%bind () = error stale Stale_input in
  let%bind () = set_shown true in
  let%bind () = settle window in
  let%bind current = E.of_thunk (fun () -> Option.value_exn !latest) in
  let%bind mounted = C.read_snapshot current >>= expect in
  let%bind () = check mounted initial in
  let%bind stale = C.replace_if_unchanged current first (single "2024-01-01") in
  let%bind () = error stale Stale_input in
  let%bind stale = C.clear code () in
  let%bind () = error stale Stale_input in
  let%bind () =
    E.of_thunk (fun () ->
      assert (
        not
          (List.exists !received ~f:(function
             | Calendar.Event.Selected _ -> true
             | _ -> false))))
  in
  let%bind closing =
    concurrent
      [ C.read_snapshot current
      ; (let%bind () = E.of_thunk (fun () -> App.Window.close window) in
         C.read_snapshot current)
      ]
  in
  let%bind () =
    match closing with
    | [ before; after ] ->
      let%bind before = expect before in
      let%bind () = check before initial in
      error after Closed
    | _ -> E.of_thunk (fun () -> failwith "Missing close results")
  in
  E.of_thunk (fun () -> completed := true)
;;

let component ~self_test ~completed window graph =
  let open B.Let_syntax in
  let shown, set_shown = B.state true graph in
  let policy, set_policy = B.state normal graph in
  let status, set_status =
    B.state "Civil dates stay dates, independent of time zones." graph
  in
  let received = ref [] in
  let config =
    let%arr policy = policy in
    Calendar.Config.create
      ~label:"Appointment date"
      ~constraints
      ~today:(day "2024-02-29")
      ~disabled:policy.disabled
      ~read_only:policy.read_only
      ()
    |> ok
  in
  let on_event =
    let%arr set_status = set_status in
    fun event ->
      E.Many
        [ (if self_test
           then E.of_thunk (fun () -> received := event :: !received)
           else E.Ignore)
        ; (match event with
           | Calendar.Event.Selected snapshot ->
             set_status
               (Sexp.to_string_hum
                  (Calendar.Selection.sexp_of_t (Calendar.Snapshot.selection snapshot)))
           | Rejected (reason, _) ->
             set_status (Sexp.to_string (Calendar.Selection_error.sexp_of_t reason))
           | Observed _ | Changed _ -> E.Ignore)
        ]
  in
  let code = C.create window ~config ~initial ~initial_month ~on_event graph in
  let span =
    C.create
      window
      ~config:
        (B.return
           (Calendar.Config.create ~mode:Range ~label:"Travel dates" ~constraints () |> ok))
      ~initial:Calendar.Selection.empty
      ~initial_month
      ~on_event
      graph
  in
  let unplaced = C.create window ~config ~initial ~initial_month graph in
  let latest = ref None in
  let started = ref false in
  let observations = B.map2 code span ~f:(fun a b -> C.snapshot a, C.snapshot b) in
  B.Edge.on_change
    observations
    ~equal:[%equal: Calendar.Snapshot.t option * Calendar.Snapshot.t option]
    ~callback:
      (let%arr code = code
       and span = span
       and unplaced = unplaced
       and set_policy = set_policy
       and set_shown = set_shown in
       fun (a, b) ->
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             latest := Some code;
             if self_test && Option.is_some a && Option.is_some b && not !started
             then (
               started := true;
               true)
             else false)
         in
         if begin_test
         then
           run_self_test
             window
             ~code
             ~span
             ~unplaced
             ~latest
             ~set_policy
             ~set_shown
             ~received
             ~completed
         else E.Ignore)
    graph;
  let%arr code = code
  and span = span
  and shown = shown
  and set_shown = set_shown
  and policy = policy
  and set_policy = set_policy
  and status = status
  and set_status = set_status in
  let report pending =
    let open E.Let_syntax in
    let%bind result = pending in
    set_status
      (match result with
       | Ok snapshot ->
         Sexp.to_string_hum
           (Calendar.Selection.sexp_of_t (Calendar.Snapshot.selection snapshot))
       | Error error -> Sexp.to_string (Calendar.Command_error.sexp_of_t error))
  in
  let gap n = Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn n) ] in
  let card title controller =
    View.column
      ~style:(gap 10.)
      [ View.text title
      ; C.view
          ~style:(Gpuio.Style.create_exn [ Width (Gpuio.Length.px_exn 340.) ])
          controller
      ; View.text
          (Option.value_map (C.snapshot controller) ~default:"Mounting…" ~f:(fun s ->
             Sexp.to_string_hum
               (Calendar.Selection.sexp_of_t (Calendar.Snapshot.selection s))))
      ]
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 18.) ])
    [ View.text "Native calendar lab"
    ; View.text "Day, month and year navigation · single dates and inclusive ranges"
    ; View.row
        ~style:(gap 20.)
        ((if shown
          then
            [ View.column
                ~style:
                  (Gpuio.Style.create_exn
                     [ Visibility (if policy.hidden then Hidden else Visible) ])
                [ card "Appointment" code ]
            ]
          else [])
         @ [ card "Travel range" span ])
    ; View.text status
    ; View.row
        ~style:(gap 8.)
        [ View.button ~on_click:(report (C.focus code)) "Focus"
        ; View.button ~on_click:(report (C.replace code initial)) "Leap day"
        ; View.button ~on_click:(report (C.clear code ())) "Clear"
        ; View.button ~on_click:(report (C.show_month code initial_month)) "February 2024"
        ; View.button ~on_click:(report (C.read_snapshot code)) "Read native state"
        ]
    ; View.row
        ~style:(gap 8.)
        [ View.button
            ~on_click:(set_policy { policy with disabled = not policy.disabled })
            (if policy.disabled then "Enable" else "Disable")
        ; View.button
            ~on_click:(set_policy { policy with read_only = not policy.read_only })
            (if policy.read_only then "Allow selection" else "Read-only")
        ; View.button
            ~on_click:(set_shown (not shown))
            (if shown then "Unmount" else "Remount")
        ; View.button ~on_click:(E.of_thunk (fun () -> App.Window.close window)) "Close"
        ]
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    ignore
      (App.open_window
         app
         ~title:"GPUIO calendar lab"
         ~width:780.
         ~height:620.
         (component ~self_test ~completed)
       |> ok
       : App.Window.t));
  if self_test
  then (
    assert !completed;
    Eio_main.run (fun env ->
      Eio.Flow.copy_string
        "GPUIO_CALENDAR_PUBLIC_OK: commands, bounds, modes, focus, guards, policy, \
         admission, remount and close\n"
        (Eio.Stdenv.stdout env)))
;;

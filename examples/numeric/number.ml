open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module N = Gpuio.Number_input
module Controller = Gpuio_eio.Number_input
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let value x = N.Value.of_float x |> ok
let domain = Gpuio.Numeric.Domain.create ~min:(-2.) ~max:8. ~step:0.5 |> ok
let draft t text = Controller.replace_draft t ~selection:End ~undo:Record text

let replace t number =
  Controller.replace_value t ~selection:End ~undo:Reset (value number)
;;

let expect = function
  | Ok value -> E.return value
  | Error error ->
    E.of_thunk (fun () -> failwith (Sexp.to_string (N.Command_error.sexp_of_t error)))
;;

let error result expected =
  E.of_thunk (fun () ->
    if not (Result.equal N.Snapshot.equal N.Command_error.equal result (Error expected))
    then
      raise_s
        [%message
          "Unexpected numeric command result"
            (result : (N.Snapshot.t, N.Command_error.t) Result.t)
            (expected : N.Command_error.t)])
;;

let check snapshot text committed =
  E.of_thunk (fun () ->
    if
      not
        (String.equal (N.Snapshot.draft snapshot) text
         && N.Value.equal (N.Snapshot.committed snapshot) committed)
    then
      raise_s
        [%message
          "Unexpected numeric snapshot"
            (snapshot : N.Snapshot.t)
            (text : string)
            (committed : N.Value.t)])
;;

let frame window =
  E.Expert.of_fun ~f:(fun ~callback ->
    App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
      E.of_thunk (fun () -> callback ()))
    |> ok)
;;

let settle window = E.bind (frame window) ~f:(fun () -> frame window)

let component ~self_test ~completed window graph =
  let open B.Let_syntax in
  let shown, set_shown = B.state true graph in
  let mount_value, set_mount_value = B.state (value 1.5) ~equal:N.Value.equal graph in
  let mount_draft, set_mount_draft = B.state_opt ~equal:N.Draft.equal graph in
  let disabled, set_disabled = B.state false graph in
  let read_only, set_read_only = B.state false graph in
  let status, set_status = B.state "Enter commits; Escape restores; arrows step" graph in
  let received = ref [] in
  let config =
    let%arr disabled = disabled
    and read_only = read_only in
    N.Config.create ~domain ~label:"Temperature" ~disabled ~read_only () |> ok
  in
  let on_event =
    B.return (fun event ->
      if self_test
      then E.of_thunk (fun () -> received := event :: !received)
      else E.Ignore)
  in
  let number = Controller.create window ~config ~initial:(value 1.5) ~on_event graph in
  let mode label step_controls =
    Controller.create
      window
      ~config:(B.return (N.Config.create ~domain ~label ~step_controls () |> ok))
      ~initial:(value 2.)
      graph
  in
  let stacked = mode "Stacked temperature" Stacked in
  let hidden = mode "Keyboard temperature" Hidden in
  (* Deliberately unplaced controller exercises Not_mounted without racing mount. *)
  let unplaced = mode "Unplaced test field" Hidden in
  (* Native mount observations can arrive in separate UI turns. The test retains
     these controller values across effects, so all three must be ready before
     capturing them in the command sequence. *)
  let observed =
    let%arr number = number
    and stacked = stacked
    and hidden = hidden in
    Controller.snapshot number, Controller.snapshot stacked, Controller.snapshot hidden
  in
  let started = ref false in
  let latest = ref None in
  B.Edge.on_change
    observed
    ~equal:[%equal: N.Snapshot.t option * N.Snapshot.t option * N.Snapshot.t option]
    ~callback:
      (let%arr number = number
       and unplaced = unplaced
       and stacked = stacked
       and hidden = hidden
       and set_shown = set_shown
       and set_mount_value = set_mount_value
       and set_mount_draft = set_mount_draft
       and set_disabled = set_disabled
       and set_read_only = set_read_only in
       fun (number_snapshot, stacked_snapshot, hidden_snapshot) ->
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             latest := Some number;
             if
               self_test
               && Option.is_some number_snapshot
               && Option.is_some stacked_snapshot
               && Option.is_some hidden_snapshot
               && not !started
             then (
               started := true;
               true)
             else false)
         in
         if not begin_test
         then E.Ignore
         else (
           let%bind result = Controller.read_snapshot unplaced in
           let%bind () = error result Not_mounted in
           let%bind initial = Controller.read_snapshot number >>= expect in
           let%bind () = check initial "1.5" (value 1.5) in
           let%bind next =
             Controller.replace_value_if_unchanged
               number
               initial
               ~selection:End
               ~undo:Reset
               (value 2.)
             >>= expect
           in
           let%bind () = check next "2" (value 2.) in
           let%bind stale =
             Controller.replace_value_if_unchanged
               number
               initial
               ~selection:End
               ~undo:Record
               (value 3.)
           in
           let%bind () = error stale Stale_revision in
           let%bind incomplete = draft number "-" >>= expect in
           let%bind () = check incomplete "-" (value 2.) in
           let%bind rejected = Controller.commit number in
           let%bind () = error rejected (Rejected Incomplete) in
           let%bind cancelled = Controller.cancel number >>= expect in
           let%bind () = check cancelled "2" (value 2.) in
           let%bind _ = draft number "99" >>= expect in
           let%bind clamped = Controller.commit number >>= expect in
           let%bind () = check clamped "8" (value 8.) in
           let%bind stepped = Controller.step number Decrease >>= expect in
           let%bind () = check stepped "7.5" (value 7.5) in
           let%bind _ = replace number 1.5 >>= expect in
           let%bind _ = draft number "3.25" >>= expect in
           let selection = Gpuio.Text_input.Selection.create ~anchor:4 ~head:1 |> ok in
           let%bind selected = Controller.select number selection >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Gpuio.Text_input.Selection.equal
                   (N.Snapshot.selection selected)
                   selection))
           in
           let%bind undone = Controller.undo number >>= expect in
           let%bind () = check undone "1.5" (value 1.5) in
           let%bind redone = Controller.redo number >>= expect in
           let%bind () = check redone "3.25" (value 1.5) in
           let%bind _ = draft number "é" >>= expect in
           let%bind invalid =
             Controller.select
               number
               (Gpuio.Text_input.Selection.create ~anchor:1 ~head:1 |> ok)
           in
           let%bind () = error invalid Invalid_selection in
           let%bind invalid = draft number "\n" in
           let%bind () = error invalid Invalid_text in
           let%bind oversized = draft number (String.make (N.max_draft_bytes + 1) '1') in
           let%bind () = error oversized Limit_exceeded in
           let%bind rejected = Controller.commit number in
           let%bind () = error rejected (Rejected Syntax) in
           let%bind _ = draft number "" >>= expect in
           let%bind rejected = Controller.commit number in
           let%bind () = error rejected (Rejected Empty_required) in
           let%bind seeded = Controller.step number Increase >>= expect in
           let%bind () = check seeded "0" (value 0.) in
           let%bind _ = replace number 1.5 >>= expect in
           let%bind () = set_disabled true in
           let%bind () = settle window in
           let%bind denied = Controller.focus number in
           let%bind () = error denied Focus_blocked in
           let%bind denied = Controller.step number Increase in
           let%bind () = error denied Disabled in
           let%bind replaced = replace number 2.5 >>= expect in
           let%bind () = check replaced "2.5" (value 2.5) in
           let%bind () = set_disabled false in
           let%bind () = set_read_only true in
           let%bind () = settle window in
           let%bind denied = Controller.step number Increase in
           let%bind () = error denied Read_only in
           let%bind _ = Controller.focus number >>= expect in
           let%bind () = set_read_only false in
           (* Other presentations use the exact same public command route. *)
           let%bind stacked = Controller.step stacked Increase >>= expect in
           let%bind () = check stacked "2.5" (value 2.5) in
           let%bind hidden = Controller.step hidden Decrease >>= expect in
           let%bind () = check hidden "1.5" (value 1.5) in
           let%bind partial = draft number "1e-" >>= expect in
           let%bind () = set_mount_value (value 4.5) in
           let%bind () =
             set_mount_draft (Some (N.Draft.of_string "different seed" |> ok))
           in
           let%bind () = settle window in
           let%bind unchanged = Controller.read_snapshot number >>= expect in
           let%bind () = check unchanged "1e-" (N.Snapshot.committed partial) in
           let%bind () =
             set_mount_draft (Some (N.Draft.of_string (N.Snapshot.draft partial) |> ok))
           in
           let%bind () = set_shown false in
           let%bind () = settle window in
           let%bind stale = Controller.read_snapshot number in
           let%bind () = error stale Stale_input in
           let%bind () = set_shown true in
           let%bind () = settle window in
           let%bind stale = Controller.read_snapshot number in
           let%bind () = error stale Stale_input in
           let%bind current = E.of_thunk (fun () -> Option.value_exn !latest) in
           let%bind mounted = Controller.read_snapshot current >>= expect in
           let%bind () = check mounted "1e-" (value 4.5) in
           let%bind cancelled = Controller.cancel current >>= expect in
           let%bind () = check cancelled "4.5" (value 4.5) in
           let%bind stale =
             Controller.replace_value_if_unchanged
               current
               initial
               ~selection:End
               ~undo:Reset
               (value 4.)
           in
           let%bind () = error stale Stale_input in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 List.exists !received ~f:(function
                   | N.Event.Rejected (Incomplete, _) -> true
                   | _ -> false));
               assert (
                 List.exists !received ~f:(function
                   | N.Event.Cancelled (Programmatic, _) -> true
                   | _ -> false));
               assert (
                 List.exists !received ~f:(function
                   | N.Event.Committed (Programmatic, _) -> true
                   | _ -> false));
               App.Window.close window)
           in
           let%bind closed = Controller.read_snapshot current in
           let%bind () = error closed Closed in
           E.of_thunk (fun () -> completed := true)))
    graph;
  let%arr number = number
  and stacked = stacked
  and hidden = hidden
  and shown = shown
  and mount_value = mount_value
  and mount_draft = mount_draft
  and set_shown = set_shown
  and disabled = disabled
  and set_disabled = set_disabled
  and read_only = read_only
  and set_read_only = set_read_only
  and status = status
  and set_status = set_status in
  let report pending =
    let open E.Let_syntax in
    let%bind result = pending in
    set_status
      (match result with
       | Ok snapshot ->
         "Revision "
         ^ Int64.to_string (N.Revision.to_int64 (N.Snapshot.revision snapshot))
       | Error error -> Sexp.to_string (N.Command_error.sexp_of_t error))
  in
  let mode ?initial ?initial_draft title controller =
    let observed =
      Option.value_map
        (Controller.snapshot controller)
        ~default:"Mounting"
        ~f:(fun snapshot ->
          "Draft: "
          ^ Sexp.to_string (String.sexp_of_t (N.Snapshot.draft snapshot))
          ^ " · Committed: "
          ^ Sexp.to_string (N.Value.sexp_of_t (N.Snapshot.committed snapshot)))
    in
    View.column
      ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ])
      [ View.text title
      ; Controller.view
          ?initial
          ?initial_draft
          ~style:
            (Gpuio.Style.create_exn
               [ Width (Gpuio.Length.px_exn 360.); Height (Gpuio.Length.px_exn 44.) ])
          controller
      ; View.text observed
      ]
  in
  let row children =
    View.row ~style:(Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn 8.) ]) children
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 16.) ])
    ([ View.text "Native numeric editors"; View.text "Bounds −2 to 8 · Step 0.5" ]
     @ (if shown
        then
          [ mode ~initial:mount_value ?initial_draft:mount_draft "Side steppers" number ]
        else [])
     @ [ View.text status
       ; row
           [ View.button ~on_click:(report (Controller.commit number)) "Commit"
           ; View.button ~on_click:(report (Controller.cancel number)) "Restore"
           ; View.button ~on_click:(report (replace number 1.5)) "Reset value"
           ; View.button ~on_click:(report (Controller.read_snapshot number)) "Read state"
           ]
       ; row
           [ View.button
               ~on_click:(set_disabled (not disabled))
               (if disabled then "Enable" else "Disable")
           ; View.button
               ~on_click:(set_read_only (not read_only))
               (if read_only then "Allow edits" else "Read-only")
           ; View.button
               ~on_click:(set_shown (not shown))
               (if shown then "Unmount" else "Mount")
           ; View.button
               ~on_click:(E.of_thunk (fun () -> App.Window.close window))
               "Close"
           ]
       ; mode "Stacked steppers" stacked
       ; mode "Hidden steppers · use arrow keys" hidden
       ])
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    ignore
      (App.open_window
         app
         ~title:"GPUIO numeric editors"
         ~width:720.
         ~height:680.
         (component ~self_test ~completed)
       |> ok
       : App.Window.t));
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_NUMBER_PUBLIC_OK: three modes, commands/events, drafts/commit/history, \
       guards, policy, atomic draft/value remount seeds, cancel to committed value, \
       close")
;;

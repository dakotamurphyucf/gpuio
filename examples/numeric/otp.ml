open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module O = Gpuio.Otp_input
module C = Gpuio_eio.Otp_input
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let digits = O.Policy.create ~length:6 () |> ok
let letters = O.Policy.create ~length:8 ~alphabet:Ascii_alphanumeric () |> ok

let value policy text =
  O.Value.of_string policy text
  |> Result.map_error ~f:(fun error -> Error.create_s (O.Input_error.sexp_of_t error))
  |> ok
;;

let expect = function
  | Ok value -> E.return value
  | Error error -> E.of_thunk (fun () -> raise_s [%sexp (error : O.Command_error.t)])
;;

let error result expected =
  E.of_thunk (fun () ->
    if not (Result.equal O.Snapshot.equal O.Command_error.equal result (Error expected))
    then
      raise_s
        [%message
          "Unexpected OTP command result"
            (result : (O.Snapshot.t, O.Command_error.t) Result.t)
            (expected : O.Command_error.t)])
;;

let check snapshot text =
  E.of_thunk (fun () ->
    assert (String.equal (O.Value.to_string (O.Snapshot.value snapshot)) text))
;;

let frame window =
  E.Expert.of_fun ~f:(fun ~callback ->
    App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
      E.of_thunk (fun () -> callback ()))
    |> ok)
;;

let settle window = E.bind (frame window) ~f:(fun () -> frame window)

let replace controller policy text =
  C.replace controller ~selection:End ~undo:Record (value policy text)
;;

(* [Effect.all] sequences requests. Start these together to exercise admission
   and close ordering before the UI loop drains native replies. *)
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

let component ~self_test ~completed window graph =
  let open B.Let_syntax in
  let shown, set_shown = B.state true graph in
  let disabled, set_disabled = B.state false graph in
  let read_only, set_read_only = B.state false graph in
  let masked, set_masked = B.state false graph in
  let status, set_status =
    B.state "Completion means the code is filled, not authenticated." graph
  in
  let received = ref [] in
  let config =
    let%arr disabled = disabled
    and read_only = read_only
    and masked = masked in
    O.Config.create
      ~policy:digits
      ~label:"Six-digit verification code"
      ~disabled
      ~read_only
      ~masked
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
           | O.Event.Complete _ ->
             set_status "Code filled. No authentication request was sent."
           | Rejected (error, _) ->
             set_status (Sexp.to_string (O.Input_error.sexp_of_t error))
           | Observed _ | Changed _ -> E.Ignore)
        ]
  in
  let code = C.create window ~config ~initial:(value digits "12") ~on_event graph in
  let alpha =
    C.create
      window
      ~config:
        (B.return
           (O.Config.create ~policy:letters ~label:"Case-sensitive recovery code" () |> ok))
      ~initial:(value letters "Ab")
      graph
  in
  let unplaced = C.create window ~config ~initial:O.Value.empty graph in
  let latest_alpha = ref None in
  B.Edge.on_change
    (B.map alpha ~f:C.snapshot)
    ~equal:(Option.equal O.Snapshot.equal)
    ~callback:
      (let%arr alpha = alpha in
       fun _ -> E.of_thunk (fun () -> latest_alpha := Some alpha))
    graph;
  let observed = B.map code ~f:C.snapshot in
  let latest = ref None in
  let started = ref false in
  B.Edge.on_change
    observed
    ~equal:(Option.equal O.Snapshot.equal)
    ~callback:
      (let%arr code = code
       and unplaced = unplaced
       and set_shown = set_shown
       and set_disabled = set_disabled
       and set_read_only = set_read_only in
       fun observation ->
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             latest := Some code;
             if self_test && Option.is_some observation && not !started
             then (
               started := true;
               true)
             else false)
         in
         if not begin_test
         then E.Ignore
         else (
           let%bind () = settle window in
           let%bind unmounted = C.read_snapshot unplaced in
           let%bind () = error unmounted Not_mounted in
           let%bind initial = C.read_snapshot code >>= expect in
           let%bind () = check initial "12" in
           (* All effects enqueue within one UI turn, before native replies drain. *)
           let%bind reads =
             concurrent (List.init 65 ~f:(fun _ -> C.read_snapshot code))
           in
           let%bind () =
             E.of_thunk (fun () ->
               assert (List.count reads ~f:Result.is_ok = 64);
               assert (
                 List.count reads ~f:(function
                   | Error O.Command_error.Busy -> true
                   | _ -> false)
                 = 1))
           in
           let%bind focused = C.focus code >>= expect in
           let%bind () = E.of_thunk (fun () -> assert (O.Snapshot.focused focused)) in
           let%bind guarded =
             C.replace_if_unchanged
               code
               focused
               ~selection:End
               ~undo:Reset
               (value digits "123456")
             >>= expect
           in
           let%bind () = check guarded "123456" in
           let%bind stale =
             C.replace_if_unchanged
               code
               focused
               ~selection:End
               ~undo:Record
               (value digits "34")
           in
           let%bind () = error stale Stale_revision in
           let%bind invalid = replace code letters "ABC" in
           let%bind () = error invalid Invalid_value in
           let%bind invalid =
             C.replace
               code
               ~selection:
                 (Select (Gpuio.Text_input.Selection.create ~anchor:4 ~head:0 |> ok))
               ~undo:Record
               (value digits "12")
           in
           let%bind () = error invalid Invalid_selection in
           let%bind next = replace code digits "654321" >>= expect in
           let%bind () = check next "654321" in
           let selected = Gpuio.Text_input.Selection.create ~anchor:5 ~head:2 |> ok in
           let%bind selection = C.select code selected >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Gpuio.Text_input.Selection.equal
                   (O.Snapshot.selection selection)
                   selected))
           in
           let%bind shortened =
             C.replace code ~selection:Preserve ~undo:Record (value digits "98")
             >>= expect
           in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Gpuio.Text_input.Selection.equal
                   (O.Snapshot.selection shortened)
                   (Gpuio.Text_input.Selection.create ~anchor:2 ~head:2 |> ok)))
           in
           let%bind undone = C.undo code >>= expect in
           let%bind () = check undone "654321" in
           let%bind redone = C.redo code >>= expect in
           let%bind () = check redone "98" in
           let%bind reset = C.clear code ~undo:Reset () >>= expect in
           let%bind () = check reset "" in
           let%bind () =
             E.of_thunk (fun () ->
               assert (not (O.Snapshot.can_undo reset || O.Snapshot.can_redo reset)))
           in
           let%bind _ = C.cancel_composition code >>= expect in
           let%bind alpha = E.of_thunk (fun () -> Option.value_exn !latest_alpha) in
           let%bind alpha_initial = C.read_snapshot alpha >>= expect in
           let%bind () = check alpha_initial "Ab" in
           let%bind alpha_next = replace alpha letters "ａｂ１２ＣＤ３４" >>= expect in
           let%bind () = check alpha_next "ab12CD34" in
           let wide = O.Policy.create ~length:32 ~alphabet:Ascii_alphanumeric () |> ok in
           let%bind invalid = replace alpha wide "123456789" in
           let%bind () = error invalid Invalid_value in
           let%bind () = set_disabled true in
           let%bind () = settle window in
           let%bind denied = C.focus code in
           let%bind () = error denied Focus_blocked in
           let%bind denied = C.undo code in
           let%bind () = error denied Disabled in
           let%bind changed = replace code digits "56" >>= expect in
           let%bind () = check changed "56" in
           let%bind () = set_disabled false in
           let%bind () = set_read_only true in
           let%bind () = settle window in
           let%bind _ = C.focus code >>= expect in
           let%bind denied = C.undo code in
           let%bind () = error denied Read_only in
           let%bind changed = C.clear code ~undo:Reset () >>= expect in
           let%bind () = check changed "" in
           let%bind () = set_read_only false in
           let%bind () = set_shown false in
           let%bind () = settle window in
           let%bind stale = C.read_snapshot code in
           let%bind () = error stale Stale_input in
           let%bind () = set_shown true in
           let%bind () = settle window in
           let%bind current = E.of_thunk (fun () -> Option.value_exn !latest) in
           let%bind mounted = C.read_snapshot current >>= expect in
           let%bind () = check mounted "12" in
           let%bind stale =
             C.replace_if_unchanged
               current
               initial
               ~selection:End
               ~undo:Record
               (value digits "22")
           in
           let%bind () = error stale Stale_input in
           let%bind stale = C.clear code ~undo:Reset () in
           let%bind () = error stale Stale_input in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 not
                   (List.exists !received ~f:(function
                      | O.Event.Complete _ -> true
                      | _ -> false))))
           in
           let%bind closing =
             concurrent
               [ C.read_snapshot current
               ; (let%bind () = E.of_thunk (fun () -> App.Window.close window) in
                  C.read_snapshot current)
               ]
           in
           (* An admitted read precedes native close; commands issued after the
              local close request are rejected immediately. *)
           let%bind () =
             match closing with
             | [ before; after ] ->
               let%bind before = expect before in
               let%bind () = check before "12" in
               error after Closed
             | _ -> E.of_thunk (fun () -> failwith "Missing concurrent results")
           in
           let%bind closed = C.read_snapshot current in
           let%bind () = error closed Closed in
           E.of_thunk (fun () -> completed := true)))
    graph;
  let%arr code = code
  and alpha = alpha
  and shown = shown
  and set_shown = set_shown
  and masked = masked
  and set_masked = set_masked
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
         "Native revision "
         ^ Int64.to_string (O.Revision.to_int64 (O.Snapshot.revision snapshot))
       | Error error -> Sexp.to_string (O.Command_error.sexp_of_t error))
  in
  let gap n = Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn n) ] in
  let details controller =
    Option.value_map (C.snapshot controller) ~default:"Mounting…" ~f:(fun snapshot ->
      sprintf
        "%d / %d characters · %s"
        (O.Value.length (O.Snapshot.value snapshot))
        (O.Policy.length (O.Snapshot.policy snapshot))
        (if Option.is_some (O.Snapshot.composition snapshot)
         then "Composing"
         else if O.Snapshot.is_complete snapshot
         then "Filled"
         else "Ready"))
  in
  let field title controller =
    View.column
      ~style:(gap 8.)
      [ View.text title
      ; C.view
          ~style:(Gpuio.Style.create_exn [ Height (Gpuio.Length.px_exn 44.) ])
          controller
      ; View.text (details controller)
      ]
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 18.) ])
    ([ View.text "Native verification inputs"
     ; View.text "One editor per code · selection, paste, history and native IME"
     ; View.button ~on_click:(E.of_thunk (fun () -> App.Window.close window)) "Close"
     ]
     @ (if shown then [ field "Six-digit code" code ] else [])
     @ [ field "Case-sensitive recovery code" alpha
       ; View.row
           ~style:(gap 8.)
           [ View.button
               ~on_click:(report (replace alpha letters "Ab12Cd34"))
               "Fill recovery sample"
           ; View.button
               ~on_click:(report (C.clear alpha ~undo:Record ()))
               "Clear recovery"
           ]
       ; View.text status
       ; View.row
           ~style:(gap 8.)
           [ View.button ~on_click:(report (replace code digits "123456")) "Fill sample"
           ; View.button ~on_click:(report (C.clear code ~undo:Record ())) "Clear"
           ; View.button ~on_click:(report (C.undo code)) "Undo"
           ; View.button ~on_click:(report (C.redo code)) "Redo"
           ]
       ; View.row
           ~style:(gap 8.)
           [ View.button
               ~on_click:(set_masked (not masked))
               (if masked then "Reveal" else "Mask")
           ; View.button
               ~on_click:(set_disabled (not disabled))
               (if disabled then "Enable" else "Disable")
           ; View.button
               ~on_click:(set_read_only (not read_only))
               (if read_only then "Allow edits" else "Read-only")
           ; View.button
               ~on_click:(set_shown (not shown))
               (if shown then "Unmount" else "Remount")
           ]
       ])
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    ignore
      (App.open_window
         app
         ~title:"GPUIO verification inputs"
         ~width:760.
         ~height:480.
         (component ~self_test ~completed)
       |> ok
       : App.Window.t));
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_OTP_PUBLIC_OK: native commands, selection/history, guards, \
       disabled/read-only, remount and close")
;;

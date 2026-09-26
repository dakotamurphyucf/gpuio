open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module N = Gpuio.Color_input
module V = Gpuio.Color_value
module C = Gpuio_eio.Color_input
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let color text = V.Value.Color (V.Rgba.of_hex text |> ok)
let initial = color "#7C6FF0"
let translucent = color "#EF6B9580"

let value_text = function
  | V.Value.Empty -> "No color"
  | Color color -> V.Rgba.to_hex color
;;

let expect = function
  | Ok snapshot -> E.return snapshot
  | Error error -> E.of_thunk (fun () -> raise_s [%sexp (error : N.Command_error.t)])
;;

let error result expected =
  E.of_thunk (fun () ->
    assert (Result.equal N.Snapshot.equal N.Command_error.equal result (Error expected)))
;;

let check snapshot value =
  E.of_thunk (fun () -> assert (V.Value.equal (N.Snapshot.value snapshot) value))
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
  ; opaque : bool
  }
[@@deriving equal]

let normal = { disabled = false; read_only = false; hidden = false; opaque = false }

let run_self_test
      window
      ~control
      ~unplaced
      ~latest
      ~set_policy
      ~set_shown
      ~received
      ~completed
  =
  let open E.Let_syntax in
  let%bind () = settle window in
  let%bind absent = C.read_snapshot unplaced in
  let%bind () = error absent Not_mounted in
  let%bind first = C.read_snapshot control >>= expect in
  let%bind () = check first initial in
  let%bind reads = concurrent (List.init 65 ~f:(fun _ -> C.read_snapshot control)) in
  let%bind () =
    E.of_thunk (fun () ->
      assert (List.count reads ~f:Result.is_ok = 64);
      assert (
        List.count reads ~f:(function
          | Error N.Command_error.Busy -> true
          | _ -> false)
        = 1))
  in
  let%bind _ = C.focus control Hex >>= expect in
  let%bind changed = C.set_if_unchanged control first translucent >>= expect in
  let%bind () = check changed translucent in
  let%bind stale = C.set_if_unchanged control first initial in
  let%bind () = error stale Stale_revision in
  let%bind () = settle window in
  let%bind () =
    E.of_thunk (fun () ->
      let current = Option.value_exn !latest |> C.snapshot |> Option.value_exn in
      assert (N.Snapshot.equal current changed))
  in
  let%bind () = set_policy { normal with opaque = true } in
  let%bind () = settle window in
  let%bind historical = C.read_snapshot control >>= expect in
  let%bind () = check historical translucent in
  let%bind () =
    E.of_thunk (fun () -> assert (not (N.Snapshot.value_allowed historical)))
  in
  let%bind denied = C.set control translucent in
  let%bind () = error denied Invalid_value in
  let%bind denied = C.focus control (Channel Alpha) in
  let%bind () = error denied Focus_blocked in
  let%bind reset = C.reset control () >>= expect in
  let%bind () = check reset initial in
  let%bind () = set_policy { normal with disabled = true } in
  let%bind () = settle window in
  let%bind denied = C.focus control Hex in
  let%bind () = error denied Focus_blocked in
  let%bind _ = C.set control translucent >>= expect in
  let%bind () = set_policy { normal with read_only = true } in
  let%bind () = settle window in
  let%bind _ = C.focus control (Channel Hue) >>= expect in
  let%bind cleared = C.clear control () >>= expect in
  let%bind () = check cleared Empty in
  let%bind () = set_policy { normal with hidden = true } in
  let%bind () = settle window in
  let%bind denied = C.focus control Hex in
  let%bind () = error denied Focus_blocked in
  let%bind reset = C.reset control () >>= expect in
  let%bind () = check reset initial in
  let%bind cancelled = C.cancel control >>= expect in
  let%bind () = E.of_thunk (fun () -> assert (N.Snapshot.equal cancelled reset)) in
  let%bind () = set_policy normal in
  let%bind () = set_shown false in
  let%bind () = settle window in
  let%bind stale = C.read_snapshot control in
  let%bind () = error stale Stale_color_input in
  let%bind () = set_shown true in
  let%bind () = settle window in
  let%bind current = E.of_thunk (fun () -> Option.value_exn !latest) in
  let%bind mounted = C.read_snapshot current >>= expect in
  let%bind () = check mounted initial in
  let%bind stale = C.set_if_unchanged current first translucent in
  let%bind () = error stale Stale_color_input in
  let%bind stale = C.clear control () in
  let%bind () = error stale Stale_color_input in
  let%bind () =
    E.of_thunk (fun () ->
      assert (
        not
          (List.exists !received ~f:(function
             | N.Event.Committed _ -> true
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
    | _ -> E.of_thunk (fun () -> failwith "Missing color close results")
  in
  E.of_thunk (fun () -> completed := true)
;;

let component ~self_test ~completed window graph =
  let open B.Let_syntax in
  let policy, set_policy = B.state normal graph in
  let shown, set_shown = B.state true graph in
  let status, set_status =
    B.state "Native precision. One color, five editable fields." graph
  in
  let config =
    let%arr policy = policy in
    N.Config.create
      ~labels:(N.Labels.english ~control:"Accent color" |> ok)
      ~palette:
        (List.map
           [ "Iris", "#7C6FF0"; "Rose glass", "#EF6B9580"; "Mint", "#54C6A2" ]
           ~f:(fun (label, hex) ->
             N.Palette_entry.create ~label ~color:(V.Rgba.of_hex hex |> ok) |> ok))
      ~alpha_policy:(if policy.opaque then Opaque_only else Allow_alpha)
      ~allow_empty:true
      ~disabled:policy.disabled
      ~read_only:policy.read_only
      ()
    |> ok
  in
  let received = ref [] in
  let on_event =
    let%arr set_status = set_status in
    fun event ->
      E.Many
        [ (if self_test
           then E.of_thunk (fun () -> received := event :: !received)
           else E.Ignore)
        ; set_status (value_text (N.Snapshot.value (N.Event.snapshot event)))
        ]
  in
  let control = C.create window ~config ~initial ~on_event graph in
  let unplaced = C.create window ~config ~initial graph in
  let latest = ref None in
  let started = ref false in
  B.Edge.on_change
    (B.map control ~f:C.snapshot)
    ~equal:[%equal: N.Snapshot.t option]
    ~callback:
      (let%arr control = control
       and unplaced = unplaced
       and set_policy = set_policy
       and set_shown = set_shown in
       fun observation ->
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             latest := Some control;
             if self_test && Option.is_some observation && not !started
             then (
               started := true;
               true)
             else false)
         in
         if begin_test
         then
           run_self_test
             window
             ~control
             ~unplaced
             ~latest
             ~set_policy
             ~set_shown
             ~received
             ~completed
         else E.Ignore)
    graph;
  let%arr control = control
  and policy = policy
  and set_policy = set_policy
  and shown = shown
  and set_shown = set_shown
  and status = status
  and set_status = set_status in
  let report pending =
    let open E.Let_syntax in
    let%bind result = pending in
    set_status
      (match result with
       | Ok snapshot -> value_text (N.Snapshot.value snapshot)
       | Error error -> Sexp.to_string (N.Command_error.sexp_of_t error))
  in
  let gap n = Gpuio.Style.create_exn [ Gap (Gpuio.Length.px_exn n) ] in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 16.) ])
    [ View.text "Color Studio"
    ; View.text "Hex · hue · saturation · lightness · alpha"
    ; View.column
        ~style:
          (Gpuio.Style.create_exn
             [ Visibility (if policy.hidden then Hidden else Visible) ])
        (if shown
         then
           [ C.view
               ~style:(Gpuio.Style.create_exn [ Width (Gpuio.Length.px_exn 380.) ])
               control
           ]
         else [])
    ; View.text status
    ; View.row
        ~style:(gap 8.)
        [ View.button ~on_click:(report (C.focus control Hex)) "Edit hex"
        ; View.button ~on_click:(report (C.set control translucent)) "Rose glass"
        ; View.button ~on_click:(report (C.reset control ())) "Reset"
        ; View.button ~on_click:(report (C.clear control ())) "Clear"
        ; View.button ~on_click:(report (C.cancel control)) "Cancel edit"
        ]
    ; View.row
        ~style:(gap 8.)
        [ View.button
            ~on_click:(set_policy { policy with opaque = not policy.opaque })
            (if policy.opaque then "Allow alpha" else "Opaque only")
        ; View.button
            ~on_click:(set_policy { policy with read_only = not policy.read_only })
            (if policy.read_only then "Allow editing" else "Read-only")
        ; View.button
            ~on_click:(set_policy { policy with disabled = not policy.disabled })
            (if policy.disabled then "Enable" else "Disable")
        ; View.button
            ~on_click:(set_shown (not shown))
            (if shown then "Unmount" else "Remount")
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
         ~title:"GPUIO Color Studio"
         ~width:620.
         ~height:600.
         (component ~self_test ~completed)
       |> ok
       : App.Window.t));
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_COLOR_CONTROLLER_OK: commands, revisions, policy, 64-request admission, \
       remount and close")
;;

open Core
module A = Signal_studio_notifications.Run_alerts
module N = Gpuio.Notification
module E = Bonsai.Effect

let handle = E.Expert.handle

let capabilities : N.Capabilities.t =
  { body = true
  ; actions = true
  ; activation = true
  ; replacement = true
  ; dismissal = true
  ; permission_request = true
  ; sound = false
  }
;;

let receipt id = N.Expert.receipt_of_wire { id; tag = "completed-run" } |> Or_error.ok_exn
let open_action = N.Action_id.of_string "open-workspace" |> Or_error.ok_exn

let backend ?(authorization = Ok N.Authorization.Authorized) ?(caps = capabilities) () =
  { A.Backend.authorization = (fun () -> E.return authorization)
  ; authorize = (fun () -> E.return authorization)
  ; capabilities = (fun () -> E.return (Ok caps))
  ; post = (fun _ -> E.return (Ok (receipt 1L)))
  ; replace = (fun _ _ -> E.return (Ok ()))
  ; dismiss = (fun _ -> E.return (Ok ()))
  ; close = Fn.id
  }
;;

let create backend =
  A.create backend ~activate:(fun () -> E.Ignore) ~on_state:ignore ~log:ignore
;;

let observe t =
  let s = A.state t in
  print_s
    [%sexp
      (s.enabled : bool)
    , (s.busy : bool)
    , (s.has_notification : bool)
    , (s.message : string)]
;;

let%expect_test "probe never opts in; unavailable and denied keep in-app completion" =
  List.iter
    [ Ok N.Authorization.Authorized; Ok Denied; Error N.Error.Unavailable ]
    ~f:(fun authorization ->
      let posts = ref 0 in
      let backend =
        { (backend ~authorization ()) with
          post =
            (fun _ ->
              incr posts;
              E.return (Ok (receipt 1L)))
        }
      in
      let t = create backend in
      handle (A.probe t);
      assert (not (A.state t).enabled);
      handle (A.enable t);
      handle (A.notify t ~run:12);
      print_s [%sexp (!posts : int), ((A.state t).enabled : bool)]);
  [%expect
    {|
    (1 true)
    (0 false)
    (0 false)
  |}]
;;

let%expect_test "single latest pending run and event before submission reply" =
  let post_reply = ref None in
  let titles = ref [] in
  let activations = ref 0 in
  let api =
    { (backend ()) with
      post =
        (fun content ->
          titles := N.title content :: !titles;
          if List.length !titles = 1
          then E.Expert.of_fun ~f:(fun ~callback -> post_reply := Some callback)
          else E.return (Ok (receipt 2L)))
    }
  in
  let t =
    A.create
      api
      ~activate:(fun () -> E.of_thunk (fun () -> incr activations))
      ~on_state:ignore
      ~log:ignore
  in
  handle (A.enable t);
  handle (A.notify t ~run:1);
  handle (A.notify t ~run:2);
  handle (A.notify t ~run:3);
  handle (A.handle_event t (Action (receipt 1L, open_action)));
  print_s [%sexp (!activations : int), ((A.state t).busy : bool)];
  Option.value_exn !post_reply (Ok (receipt 1L));
  print_s [%sexp (List.rev !titles : string list), (!activations : int)];
  handle (A.handle_event t (Activated (receipt 1L)));
  print_s [%sexp (!activations : int), ((A.state t).has_notification : bool)];
  handle (A.handle_event t (Activated (receipt 2L)));
  print_s [%sexp (!activations : int), ((A.state t).has_notification : bool)];
  [%expect
    {|
    (0 true)
    (("Signal Studio \194\183 Run 01 complete"
      "Signal Studio \194\183 Run 03 complete")
     1)
    (1 true)
    (2 false)
  |}]
;;

let%expect_test "replacement policy and capability-specific content" =
  List.iter [ true; false ] ~f:(fun replacement ->
    let calls = ref [] in
    let caps = { capabilities with replacement; body = false; actions = false } in
    let api =
      { (backend ~caps ()) with
        post =
          (fun content ->
            assert (String.is_empty (N.body content) && List.is_empty (N.actions content));
            calls := "post" :: !calls;
            E.return (Ok (receipt 1L)))
      ; replace =
          (fun _ _ ->
            calls := "replace" :: !calls;
            E.return (Ok ()))
      ; dismiss =
          (fun _ ->
            calls := "dismiss" :: !calls;
            E.return (Ok ()))
      }
    in
    let t = create api in
    handle (A.enable t);
    handle (A.notify t ~run:1);
    handle (A.notify t ~run:2);
    print_s [%sexp (List.rev !calls : string list)]);
  [%expect
    {|
    (post replace)
    (post dismiss post)
  |}]
;;

let%expect_test "close fences late success and pending event, invalid run never posts" =
  let reply = ref None in
  let posts = ref 0
  and closes = ref 0 in
  let api =
    { (backend ()) with
      post =
        (fun _ ->
          incr posts;
          E.Expert.of_fun ~f:(fun ~callback -> reply := Some callback))
    ; close = (fun () -> incr closes)
    }
  in
  let t =
    A.create
      api
      ~activate:(fun () -> failwith "closed action activated")
      ~on_state:ignore
      ~log:ignore
  in
  handle (A.enable t);
  handle (A.notify t ~run:101);
  assert (!posts = 0);
  handle (A.notify t ~run:1);
  handle (A.handle_event t (Activated (receipt 1L)));
  A.close t;
  A.close t;
  Option.value_exn !reply (Ok (receipt 1L));
  handle (A.notify t ~run:2);
  observe t;
  print_s [%sexp (!posts : int), (!closes : int)];
  [%expect
    {|
    (false false false "The alert service is closed.")
    (1 1)
  |}]
;;

let%expect_test
    "submission failures and service loss do not keep automatic alerts enabled"
  =
  List.iter [ N.Error.Denied; Unavailable; Native_failure; Not_ready ] ~f:(fun error ->
    let posts = ref 0 in
    let api =
      { (backend ()) with
        post =
          (fun _ ->
            incr posts;
            E.return (Error error))
      }
    in
    let t = create api in
    handle (A.enable t);
    handle (A.notify t ~run:1);
    handle (A.notify t ~run:2);
    print_s
      [%sexp
        (error : N.Error.t)
      , (!posts : int)
      , ((A.state t).enabled : bool)
      , ((A.state t).has_notification : bool)]);
  let t = create (backend ()) in
  handle (A.enable t);
  handle (A.notify t ~run:1);
  assert (A.state t).has_notification;
  handle (A.handle_event t (Failed Native_failure));
  observe t;
  [%expect
    {|
    (Denied 1 false false)
    (Unavailable 1 false false)
    (Native_failure 1 false false)
    (Not_ready 1 false false)
    (false false false
     "The desktop could not process this alert. Run results are still available.")
  |}]
;;

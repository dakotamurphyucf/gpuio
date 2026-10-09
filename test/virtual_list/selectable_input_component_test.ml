open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module G = Gpuio
module V = Gpuio_bonsai.Virtual_list
module R = G.Reconciler
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = G.Key.of_int
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let config = V.Config.create ~max_active:1 ~height:(Fixed 24.) () |> ok

let semantics =
  G.Accessibility.create ~role:(List_box true) ~label:"Managed choices" () |> ok
;;

let query ?(mode = G.Text_input.Mode.Single_line) () =
  G.View.text_input
    ~controller:(G.Key.of_string_exn "query")
    ~config:(G.Text_input.Config.create ~mode ~label:"Search" () |> ok)
    ~on_event:(fun _ -> E.Ignore)
    ()
  |> ok
;;

let input q ?(after = false) ?(busy = false) seen =
  V.Input.create
    ~epoch:(G.Key.of_string_exn "search-1")
    ~cursor:0
    ~query:(G.Key.of_string_exn "query")
    ~before:(if after then [] else [ q ])
    ~after:(if after then [ q ] else [])
    ~busy
    ~on_input:(fun request -> E.of_thunk (fun () -> seen := request :: !seen))
    ()
  |> ok
;;

let create component =
  Bonsai_driver.create
    ~action_history:Release_after_flush
    ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
    component
;;

let result d =
  Bonsai_driver.flush d;
  Bonsai_driver.result d |> ok
;;

let display d =
  Bonsai_driver.trigger_lifecycles d;
  ignore (result d : int V.Output.t)
;;

let owner output =
  (G.View.Expert.describe (V.Output.view output)).children
  |> List.find_exn ~f:(fun view ->
    G.View.Expert.Kind.equal (G.View.Expert.describe view).kind Virtual_list)
  |> G.View.Expert.describe
;;

let payload output = (owner output).virtual_list |> Option.value_exn

let ops update =
  match R.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let frame r d =
  let output = result d in
  let update = R.prepare r ~theme:G.Theme.default (Some (V.Output.view output)) |> ok in
  R.accept r update |> ok;
  display d;
  ops update
;;

let observe d =
  let viewport : G.Virtual_list.Viewport.t =
    { visible_first = 0
    ; visible_last = 1
    ; requested = [ key 0 ]
    ; pinned = []
    ; anchor = Some (key 0, 0.)
    ; following_tail = false
    ; at_start = true
    ; at_end = false
    ; budget_exhausted = false
    }
  in
  Bonsai_driver.schedule_event
    d
    (Option.value_exn (payload (result d)).on_viewport viewport)
;;

let rec button view =
  let description = G.View.Expert.describe view in
  match description.on_click with
  | Some callback -> Some (description.text, callback)
  | None -> List.find_map description.children ~f:button
;;

let count d = button (V.Output.view (result d)) |> Option.value_exn

let render_row ~key:row_key ~data:_ ~lifetime:_ graph =
  let count, bump =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let open B.Let_syntax in
  let%arr key = row_key
  and count = count
  and bump = bump in
  let item = G.Accessibility.Option_item.create ~index:key ~count:3 () |> ok in
  G.View.with_accessibility
    (G.View.column [ G.View.button ~on_click:(fun () -> bump ()) (Int.to_string count) ])
    (G.Accessibility.create
       ~role:(Option_item item)
       ~label:("Option " ^ Int.to_string key)
       ()
     |> ok)
  |> ok
;;

let%expect_test
    "query slots preserve native owners and transient rows while input maps collection \
     keys"
  =
  let seen = ref [] in
  let q = query () in
  let input_var = B.Expert.Var.create (input q seen) in
  let generation = B.Expert.Var.create 0L in
  let config_var = B.Expert.Var.create config in
  let driver =
    create (fun graph ->
      V.component_with_config
        (module Int)
        (B.return (G.List_collection.of_alist (module Int) [ 0, (); 1, (); 2, () ] |> ok))
        ~row_key:key
        ~config:(B.Expert.Var.value config_var)
        ~input:(B.Expert.Var.value input_var)
        ~generation:(B.Expert.Var.value generation)
        ~accessibility:(B.return semantics)
        ~render_row
        graph)
  in
  let reconciler = R.create window in
  let initial = frame reconciler driver in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (n, Virtual_list, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let query_node =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (n, Input, _, _) -> Some n
      | _ -> None)
  in
  let cfg =
    List.find_map_exn initial ~f:(function
      | W.Op.Set_list_input (_, Some c) -> Some c
      | _ -> None)
  in
  assert (Option.equal Gpuio_protocol.Node_id.equal cfg.query (Some query_node));
  assert (V.Output.active_rows (result driver) = 0);
  observe driver;
  ignore (frame reconciler driver : W.Op.t list);
  Bonsai_driver.schedule_event driver ((snd (count driver)) ());
  ignore (frame reconciler driver : W.Op.t list);
  assert (String.equal (fst (count driver)) "1");
  let old_controller = V.Output.controller (result driver) in
  let _, old_callback = Option.value_exn (payload (result driver)).list_input in
  let old_effect = old_callback (G.List_input.Confirm (key 0, Primary)) in
  B.Expert.Var.set input_var (input q ~after:true ~busy:true seen);
  let moved = frame reconciler driver in
  assert (
    not
      (List.exists moved ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  let cfg =
    List.find_map_exn moved ~f:(function
      | Set_list_input (_, Some c) -> Some c
      | _ -> None)
  in
  assert (Int64.equal cfg.generation 1L && cfg.busy);
  assert (String.equal (fst (count driver)) "1");
  let delivered =
    R.dispatch
      reconciler
      (W.Event.List_input
         (window, node, handler, R.revision reconciler, 1L, Confirm (1L, Secondary)))
    |> Option.value_exn
  in
  Bonsai_driver.schedule_event driver delivered;
  ignore (frame reconciler driver : W.Op.t list);
  (match !seen with
   | [ G.List_input.Confirm (0, Secondary) ] -> ()
   | _ -> assert false);
  B.Expert.Var.set
    config_var
    (V.Config.horizontal ~max_active:1 ~width:(Fixed 24.) () |> ok);
  let horizontal = frame reconciler driver in
  assert (
    not
      (List.exists horizontal ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  assert (String.equal (fst (count driver)) "1");
  B.Expert.Var.set generation 1L;
  ignore (frame reconciler driver : W.Op.t list);
  seen := [];
  Bonsai_driver.schedule_event driver old_effect;
  Bonsai_driver.schedule_event driver (V.Controller.jump_to_latest old_controller);
  let stale = frame reconciler driver in
  assert (List.is_empty !seen);
  assert (
    not
      (List.exists stale ~f:(function
         | W.Op.Scroll_list _ -> true
         | _ -> false)));
  assert (
    Option.is_none
      (R.dispatch reconciler (W.Event.List_input (window, node, handler, 1L, 1L, Cancel))));
  observe driver;
  ignore (frame reconciler driver : W.Op.t list);
  assert (String.equal (fst (count driver)) "0");
  R.close reconciler;
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| |}]
;;

let%expect_test "query descriptors reject ambiguous, nested and multiline editors" =
  let key = G.Key.of_string_exn "query" in
  let q = query () in
  List.iter
    [ [ q; q ]; [ G.View.column [ q ] ]; [ query ~mode:Multiline () ]; [] ]
    ~f:(fun before ->
      assert (
        Result.is_error
          (V.Input.create
             ~epoch:key
             ~query:key
             ~before
             ~on_input:(fun (_ : int G.List_input.t) -> E.Ignore)
             ())));
  assert (
    Result.is_ok
      (V.Input.create
         ~epoch:key
         ~before:[ G.View.text "Empty results" ]
         ~on_input:(fun (_ : int G.List_input.t) -> E.Ignore)
         ()));
  [%expect {| |}]
;;

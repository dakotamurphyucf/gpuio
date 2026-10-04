open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module G = Gpuio
module C = G.List_collection
module P = Gpuio_eio.List_search
module L = Gpuio_bonsai.Selectable_list
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox

let ok = Or_error.ok_exn

let%expect_test
    "search results and streamed updates preserve Bonsai preferences until source \
     replacement"
  =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:16 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let clock = Eio_mock.Clock.Mono.make () in
      let initial = C.of_alist (module Int) [ 1, "one"; 2, "two" ] |> ok in
      let search =
        P.create ~scope ~clock ~debounce:Time_ns.Span.zero initial ~search:(fun request ->
          if String.equal (P.Request.query request) "remote"
          then Ok { P.Page.upsert = [ 3, "remote" ]; visible = [ 3 ] }
          else Ok { P.Page.upsert = []; visible = [ 2 ] })
        |> ok
      in
      let driver =
        Bonsai_driver.create
          ~action_history:Release_after_flush
          ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
          (fun graph ->
             let snapshot = P.value search in
             let source = B.map snapshot ~f:P.Snapshot.items in
             let identity = B.map source ~f:C.identity |> B.cutoff ~equal:phys_equal in
             let visible =
               B.map snapshot ~f:P.Snapshot.visible |> B.cutoff ~equal:phys_equal
             in
             let open B.Let_syntax in
             let layout =
               let%arr identity = identity
               and visible = visible in
               G.List_selection.Catalog.create identity ~visible ()
               |> ok
               |> fun catalog -> G.List_rows.Layout.create catalog () |> ok
             in
             let interaction =
               let%arr snapshot = snapshot in
               L.Interaction.create
                 ~epoch:(P.Snapshot.epoch snapshot)
                 ~mode:Multiple
                 ~busy:(P.Snapshot.is_busy snapshot)
                 ~disabled:(P.Snapshot.is_stale snapshot)
                 ()
             in
             L.component
               source
               ~layout
               ~interaction
               ~config:
                 (B.return
                    (G.Virtual_list.Config.create ~max_active:1 ~height:(Fixed 32.) ()
                     |> ok))
               ~label:"Search integration"
               ~item_label:(fun ~key:_ value -> value)
               ~initial_selected:(B.return [ 1 ])
               graph)
      in
      let result () =
        Bonsai_driver.flush driver;
        Bonsai_driver.result driver |> ok
      in
      let display () =
        ignore (result () : (int, string, Int.comparator_witness) L.Output.t);
        Bonsai_driver.trigger_lifecycles driver;
        ignore (result () : (int, string, Int.comparator_witness) L.Output.t)
      in
      let settle () =
        for _ = 1 to 5 do
          Eio.Fiber.yield ();
          List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())
        done;
        display ()
      in
      let run event =
        Bonsai_driver.schedule_event driver event;
        display ()
      in
      let selected () =
        G.List_selection.selected (L.Output.state (result ()))
        |> List.map ~f:C.Item_ref.key
      in
      let query text =
        P.set_query search ~source:(P.Snapshot.source_id (P.snapshot search)) text |> ok;
        display ()
      in
      display ();
      let controller = L.Output.controller (result ()) in
      let old_target = L.Output.target (result ()) 1 |> Option.value_exn in
      query "two";
      run (L.Controller.clear_selection controller);
      assert (List.equal Int.equal (selected ()) [ 1 ]);
      (* Pending results disable input. *)
      settle ();
      assert (List.equal Int.equal (selected ()) [ 1 ]);
      run
        (L.Controller.focus
           controller
           (L.Output.target (result ()) 2 |> Option.value_exn));
      query "remote";
      settle ();
      assert (List.equal Int.equal (selected ()) [ 1 ]);
      assert (C.contains_ref (P.Snapshot.items (P.snapshot search)) old_target);
      assert (
        G.List_selection.cursor (L.Output.state (result ()))
        |> Option.value_exn
        |> C.Item_ref.key
        = 3);
      let remote = L.Output.target (result ()) 3 |> Option.value_exn in
      run (L.Controller.select controller remote Toggle);
      assert (List.equal Int.equal (selected ()) [ 1; 3 ]);
      let source = P.Snapshot.items (P.snapshot search) in
      P.update_source search ~refresh:false (C.set source ~key:3 ~data:"streamed" |> ok)
      |> ok;
      display ();
      assert (List.equal Int.equal (selected ()) [ 1; 3 ]);
      P.update_source search (C.of_alist (module Int) [ 1, "new one"; 2, "new two" ] |> ok)
      |> ok;
      settle ();
      assert (List.equal Int.equal (selected ()) [ 1 ]);
      run (L.Controller.clear_selection controller);
      assert (List.equal Int.equal (selected ()) [ 1 ]);
      (* Old source controller retired. *)
      run (L.Controller.clear_selection (L.Output.controller (result ())));
      assert (List.is_empty (selected ()));
      P.close search;
      Bonsai_driver.Expert.invalidate_observers driver;
      Scope.cancel scope;
      Inbox.close inbox));
  [%expect {| |}]
;;

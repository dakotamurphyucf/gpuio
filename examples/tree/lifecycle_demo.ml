open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Loader = Gpuio_eio.Tree_loading
module L = Gpuio.Tree_loading
module T = Gpuio.Tree
module S = Gpuio.Tree_state
module R = Gpuio.Tree_rows
module W = Gpuio_bonsai.Tree
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let id name = T.Id.of_string name |> ok
let deep index = id ("deep-" ^ Int.to_string index)
let branch = id "work"
let leaf = id "loaded"

let initial () =
  let chain =
    List.init T.max_depth ~f:(fun index ->
      let children =
        if index + 1 = T.max_depth
        then T.Children.Leaf
        else Branch { ids = [ deep (index + 1) ]; next = End }
      in
      deep index, T.Node.create ~label:(sprintf "Level %d" (index + 1)) ~children () |> ok)
  in
  let work =
    T.Node.create ~label:"Lazy work" ~children:(Branch { ids = []; next = More None }) ()
    |> ok
  in
  T.create ~roots:[ deep 0; branch ] ((branch, work) :: chain) |> ok
;;

module Producer = struct
  module Behavior = struct
    type t =
      | Wait
      | Fail
      | Complete
  end

  type t =
    { mutable behavior : Behavior.t
    ; mutable started : int
    ; mutable active : int
    ; mutable finished : int
    ; mutable peak : int
    }

  let create () = { behavior = Wait; started = 0; active = 0; finished = 0; peak = 0 }

  let load t request =
    assert (T.Id.equal (L.Request.parent request) branch);
    t.started <- t.started + 1;
    t.active <- t.active + 1;
    t.peak <- Int.max t.peak t.active;
    Exn.protect
      ~f:(fun () ->
        match t.behavior with
        | Wait -> Eio.Fiber.await_cancel ()
        | Fail -> Or_error.error_string "Deliberate retryable page failure"
        | Complete ->
          Ok
            { L.Page.roots = [ leaf ]
            ; nodes = [ leaf, T.Node.create ~label:"Loaded item" ~children:Leaf () |> ok ]
            ; next = End
            })
      ~finally:(fun () ->
        t.active <- t.active - 1;
        t.finished <- t.finished + 1)
  ;;
end

let config =
  Gpuio.Virtual_list.Config.create ~height:(Fixed 28.) ~overscan:0. ~max_active:8 () |> ok
;;

let run () =
  App.run ~exit_on_last_window:false (fun env app ->
    let clock = Eio.Stdenv.clock env in
    let producer = Producer.create () in
    let loader_ref = ref None in
    let observed = ref None in
    let mounted = ref 0 in
    let unmounted = ref 0 in
    let window =
      App.open_window
        app
        ~title:"GPUIO — Tree lifecycle"
        ~width:700.
        ~height:420.
        (fun window ->
           (* This factory runs before graph construction. The window scope owns
              producers, independently of transient row lifetimes. *)
           let loader =
             Loader.create
               ~scope:(App.Window.scope window)
               (initial ())
               ~load:(Producer.load producer)
             |> ok
           in
           loader_ref := Some loader;
           fun graph ->
             let output =
               W.component
                 (Loader.value loader)
                 ~config
                 ~label:"Lifecycle workload"
                 ~loading:(B.return (Loader.controls loader))
                 ~style:
                   (B.return
                      (Gpuio.Style.create_exn
                         [ Grow 1.
                         ; Min_height (Gpuio.Length.px_exn 0.)
                         ; Width (Gpuio.Length.percent_exn 100.)
                         ]))
                 ~render_item:(fun ~target:_ ~item ~controller:_ ~lifetime:_ graph ->
                   B.Edge.lifecycle
                     ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr mounted)))
                     ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr unmounted)))
                     graph;
                   B.map item ~f:(fun item -> View.text (T.Node.label item.node)))
                 graph
             in
             let open B.Let_syntax in
             B.Edge.after_display
               (let%arr output = output in
                E.of_thunk (fun () -> observed := Some (ok output)))
               graph;
             let%arr output = output in
             View.column
               ~style:
                 (Gpuio.Style.create_exn
                    [ Width (Gpuio.Length.percent_exn 100.)
                    ; Height (Gpuio.Length.percent_exn 100.)
                    ; Padding (Gpuio.Length.px_exn 16.)
                    ; Gap (Gpuio.Length.px_exn 8.)
                    ])
               [ View.text "Deep reveal · scoped loading · cancellation"
               ; W.Output.view (ok output)
               ])
      |> ok
    in
    let loader = Option.value_exn !loader_ref in
    Support.start (App.scope app) (fun () ->
      let wait label predicate =
        try
          Eio.Time.with_timeout_exn clock 15. (fun () ->
            while not (predicate ()) do
              Eio.Time.sleep clock 0.01
            done)
        with
        | Eio.Time.Timeout ->
          failwithf
            "tree lifecycle timeout: %s (started=%d active=%d finished=%d mounted=%d/%d) \
             %s"
            label
            producer.started
            producer.active
            producer.finished
            !mounted
            !unmounted
            (Sexp.to_string_hum
               [%sexp
                 (App.Window.snapshot window : Gpuio.Window.Snapshot.t option)
               , (Option.map !observed ~f:(fun output -> W.Output.viewport output)
                  : Gpuio.Virtual_list.Viewport.t option option)])
            ()
      in
      let output () = Option.value_exn !observed in
      let sync f = Support.perform (App.scope app) (E.of_thunk f) in
      let with_output f = Support.perform (App.scope app) (f (output ())) in
      let target output key = W.Output.target output key |> ok in
      let expanded key value =
        with_output (fun output ->
          W.Controller.set_expanded (W.Output.controller output) (target output key) value)
      in
      let reveal ?(focus = false) key =
        with_output (fun output ->
          W.Controller.reveal (W.Output.controller output) ~focus (target output key))
      in
      let snapshot () = Loader.snapshot loader in
      let tree () = L.Snapshot.tree (snapshot ()) in
      let status () = L.Snapshot.status (snapshot ()) branch in
      let focused key =
        let output = output () in
        Option.exists
          (R.item_key (W.Output.projection output) key)
          ~f:(fun row ->
            Option.exists (W.Output.viewport output) ~f:(fun viewport ->
              List.mem viewport.pinned (R.Key.to_view_key row) ~equal:Gpuio.Key.equal))
      in
      let frame () =
        let promise, resolver = Eio.Promise.create () in
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> ok;
        Eio.Time.with_timeout_exn clock 5. (fun () -> Eio.Promise.await promise)
      in
      wait "initial rows" (fun () ->
        Option.exists !observed ~f:(fun output -> W.Output.active_rows output > 0));
      reveal ~focus:true (deep (T.max_depth - 1));
      wait "deep native focus" (fun () -> focused (deep (T.max_depth - 1)));
      assert (W.Output.active_rows (output ()) <= 8);
      assert (
        (T.position (tree ()) (deep (T.max_depth - 1)) |> Option.value_exn).depth
        = T.max_depth);
      ignore
        (Support.perform
           (App.scope app)
           (E.map
              (App.Window.command window (Resize (520., 320.)))
              ~f:(fun result ->
                match result with
                | Ok _ -> ()
                | Error error -> raise_s [%sexp (error : Gpuio.Window.Error.t)]))
         : unit);
      frame ();
      wait "resized geometry" (fun () ->
        Option.exists (App.Window.snapshot window) ~f:(fun snapshot ->
          Float.(
            abs (snapshot.content_width -. 520.) < 1.
            && abs (snapshot.content_height -. 320.) < 1.)));
      assert (W.Output.active_rows (output ()) <= 8);
      reveal ~focus:true branch;
      wait "lazy branch focus" (fun () -> focused branch);
      expanded branch true;
      wait "first running load" (fun () -> producer.active = 1);
      expanded branch false;
      wait "collapse cancellation" (fun () ->
        producer.active = 0 && producer.finished = 1);
      assert (Option.is_none (T.find (tree ()) leaf));
      assert (
        Option.exists (status ()) ~f:(function
          | Ready -> true
          | Queued | Loading | End | Failed _ -> false));
      expanded branch true;
      wait "second running load" (fun () -> producer.active = 1 && producer.started = 2);
      with_output (fun output ->
        W.Controller.set_selected (W.Output.controller output) (target output branch) true);
      wait "selected branch" (fun () -> S.is_selected (W.Output.state (output ())) branch);
      let stale =
        W.Controller.reveal
          (W.Output.controller (output ()))
          ~focus:true
          (target (output ()) branch)
      in
      sync (fun () ->
        let current = tree () in
        T.replace
          current
          ~roots:[ deep 0 ]
          (List.filter (T.to_alist current) ~f:(fun (key, _) ->
             not (T.Id.equal key branch)))
        |> ok
        |> Loader.update loader
        |> ok);
      Support.perform (App.scope app) stale;
      wait "deletion cancellation and selection repair" (fun () ->
        producer.active = 0
        && producer.finished = 2
        && not (S.is_selected (W.Output.state (output ())) branch));
      sync (fun () ->
        producer.behavior <- Fail;
        Loader.reset loader (initial ()) |> ok);
      wait "source reset" (fun () ->
        L.Snapshot.same_generation
          (R.source (W.Output.projection (output ())))
          (snapshot ()));
      Support.perform (App.scope app) stale;
      reveal ~focus:true branch;
      wait "new branch focus" (fun () -> focused branch);
      expanded branch true;
      wait "retryable failure" (fun () ->
        Option.exists (status ()) ~f:(function
          | Failed _ -> true
          | Ready | Queued | Loading | End -> false));
      frame ();
      frame ();
      assert (producer.started = 3 && producer.active = 0);
      sync (fun () -> producer.behavior <- Complete);
      with_output (fun output ->
        W.Controller.retry (W.Output.controller output) (target output branch));
      wait "explicit retry completion" (fun () -> Option.is_some (T.find (tree ()) leaf));
      wait "loaded projection" (fun () ->
        Or_error.is_ok (W.Output.target (output ()) leaf));
      reveal ~focus:true leaf;
      wait "loaded child native focus" (fun () -> focused leaf);
      assert (producer.started = 4 && producer.active = 0 && producer.peak = 1);
      assert (W.Output.active_rows (output ()) <= 8);
      sync (fun () ->
        producer.behavior <- Wait;
        Loader.reset loader (initial ()) |> ok);
      wait "last reset" (fun () ->
        L.Snapshot.same_generation
          (R.source (W.Output.projection (output ())))
          (snapshot ()));
      expanded branch true;
      wait "window-owned producer" (fun () -> producer.active = 1 && producer.started = 5);
      sync (fun () -> App.Window.close window);
      wait "window close releases rows and producer" (fun () ->
        App.Window.is_closed window
        && producer.active = 0
        && producer.finished = 5
        && !mounted = !unmounted);
      assert (not (Scope.is_active (App.Window.scope window)));
      assert (
        L.Snapshot.running_count (snapshot ()) = 0
        && L.Snapshot.queued_count (snapshot ()) = 0);
      Eio.Flow.copy_string
        "TREE_LIFECYCLE_PASS depth=128 native_focus=true resize=true \
         collapse_cancel=true deletion_cancel=true selection_repair=true \
         stale_reveal=true explicit_retry=true window_cleanup=true\n"
        (Eio.Stdenv.stdout env);
      App.shutdown app))
;;

open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module T = Gpuio.Tree
module S = Gpuio.Tree_state
module R = Gpuio.Tree_rows
module P = Gpuio_eio.Tree_loading
module V = Gpuio_bonsai.Tree_rows
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module View = Gpuio.View

let ok = Or_error.ok_exn
let id text = T.Id.of_string text |> ok
let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())

let settle inbox =
  for _ = 1 to 10 do
    Eio.Fiber.yield ();
    drain inbox
  done
;;

let rec list view =
  let description = View.Expert.describe view in
  match description.virtual_list with
  | Some list -> list
  | None -> list (List.hd_exn description.children)
;;

let%expect_test
    "Bonsai viewport drives scoped loading; collapse and unmount have distinct lifetimes"
  =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:16 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let root =
        T.Node.create ~label:"Folder" ~children:(Branch { ids = []; next = More None }) ()
        |> ok
      in
      let tree = T.create ~roots:[ id "root" ] [ id "root", root ] |> ok in
      let gate, release = Eio.Promise.create () in
      let started = ref 0
      and cancelled = ref 0 in
      let loader =
        P.create ~scope tree ~load:(fun _ ->
          Int.incr started;
          (try Eio.Promise.await gate with
           | Eio.Cancel.Cancelled _ as exn ->
             Int.incr cancelled;
             raise exn);
          Ok
            { P.Page.roots = [ id "child" ]
            ; nodes = [ id "child", T.Node.create ~label:"Child" ~children:Leaf () |> ok ]
            ; next = End
            })
        |> ok
      in
      let state = B.Expert.Var.create (S.create tree ~expanded:[ id "root" ] () |> ok) in
      let shown = B.Expert.Var.create true in
      let config =
        Gpuio.Virtual_list.Config.create ~height:(Fixed 24.) ~max_active:8 () |> ok
      in
      let driver =
        Bonsai_driver.create
          ~action_history:Release_after_flush
          ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
          (fun graph ->
             let members =
               B.map (B.Expert.Var.value shown) ~f:(fun shown ->
                 if shown then Int.Map.singleton 0 () else Int.Map.empty)
             in
             B.assoc
               (module Int)
               members
               ~f:(fun _ _ graph ->
                 V.component
                   (P.value loader)
                   ~state:(B.Expert.Var.value state)
                   ~config
                   ~loading:(B.return (P.controls loader))
                   ~render_row:(fun ~key:_ ~data ~lifetime:_ _ ->
                     B.map data ~f:(function
                       | R.Row.Item item -> View.text (T.Node.label item.node)
                       | Boundary _ -> View.text "Loading"))
                   graph)
               graph)
      in
      let flush () =
        Bonsai_driver.flush driver;
        Bonsai_driver.result driver
      in
      let display () =
        ignore (flush () : _ Int.Map.t);
        Bonsai_driver.trigger_lifecycles driver;
        flush ()
      in
      let output () = Map.find_exn (flush ()) 0 |> ok in
      let observe () =
        let output = output () in
        let rows = R.collection (V.Output.projection output) in
        let requested =
          Gpuio.List_collection.keys rows |> List.map ~f:R.Key.to_view_key
        in
        let viewport : Gpuio.Virtual_list.Viewport.t =
          { visible_first = 0
          ; visible_last = Gpuio.List_collection.length rows
          ; requested
          ; pinned = []
          ; anchor = None
          ; following_tail = false
          ; at_start = true
          ; at_end = true
          ; budget_exhausted = false
          }
        in
        Bonsai_driver.schedule_event
          driver
          (Option.value_exn (list (V.Output.view output)).on_viewport viewport)
      in
      ignore (display () : _ Int.Map.t);
      assert (!started = 0);
      observe ();
      ignore (display () : _ Int.Map.t);
      settle inbox;
      assert (!started = 1 && P.Snapshot.running_count (P.snapshot loader) = 1);
      B.Expert.Var.set state (S.with_expanded (B.Expert.Var.get state) tree [] |> ok);
      ignore (display () : _ Int.Map.t);
      settle inbox;
      assert (!cancelled = 1 && P.Snapshot.running_count (P.snapshot loader) = 0);
      assert (T.length (P.Snapshot.tree (P.snapshot loader)) = 1);
      B.Expert.Var.set
        state
        (S.with_expanded (B.Expert.Var.get state) tree [ id "root" ] |> ok);
      ignore (display () : _ Int.Map.t);
      observe ();
      ignore (display () : _ Int.Map.t);
      settle inbox;
      assert (!started = 2);
      B.Expert.Var.set shown false;
      assert (Map.is_empty (display ()));
      settle inbox;
      assert (!cancelled = 1 && P.Snapshot.running_count (P.snapshot loader) = 1);
      Eio.Promise.resolve release ();
      settle inbox;
      assert (T.length (P.Snapshot.tree (P.snapshot loader)) = 2);
      assert (P.Snapshot.running_count (P.snapshot loader) = 0);
      B.Expert.Var.set shown true;
      ignore (display () : _ Int.Map.t);
      assert (
        Gpuio.List_collection.length (R.collection (V.Output.projection (output ()))) = 2);
      assert (V.Output.active_rows (output ()) = 0);
      observe ();
      ignore (display () : _ Int.Map.t);
      assert (V.Output.active_rows (output ()) = 2);
      assert (!started = 2);
      B.Expert.Var.set shown false;
      ignore (display () : _ Int.Map.t);
      Scope.cancel scope;
      Inbox.close inbox;
      Bonsai_driver.Expert.invalidate_observers driver));
  print_endline
    "viewport starts load; collapse cancels; unmounted view permits data completion; \
     remount reuses data";
  [%expect
    {| viewport starts load; collapse cancels; unmounted view permits data completion; remount reuses data |}]
;;

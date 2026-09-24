open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module C = Gpuio.List_collection
module V = Gpuio_bonsai.Virtual_list
module View = Gpuio.View

let key = Gpuio.Key.of_int
let config = V.Config.create ~max_active:4 ~height:(Estimated 80.) () |> Or_error.ok_exn
let source rows = C.of_alist (module Int) rows |> Or_error.ok_exn

let create component =
  Bonsai_driver.create
    ~action_history:Release_after_flush
    ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
    component
;;

let result driver =
  Bonsai_driver.flush driver;
  Bonsai_driver.result driver |> Or_error.ok_exn
;;

let display driver =
  Bonsai_driver.trigger_lifecycles driver;
  ignore (result driver : int V.Output.t)
;;

let list output =
  (View.Expert.describe (V.Output.view output)).children
  |> List.hd_exn
  |> View.Expert.describe
;;

let payload output = (list output).virtual_list |> Option.value_exn

let observed ?(pins = []) keys : Gpuio.Virtual_list.Viewport.t =
  { visible_first = 0
  ; visible_last = List.length keys
  ; requested = List.map keys ~f:key
  ; pinned = List.map pins ~f:key
  ; anchor = Option.map (List.hd keys) ~f:(fun key_ -> key key_, 0.)
  ; following_tail = false
  ; at_start = false
  ; at_end = false
  ; budget_exhausted = false
  }
;;

let observe driver ?pins keys =
  let output = result driver in
  let action = Option.value_exn (payload output).on_viewport (observed ?pins keys) in
  Bonsai_driver.schedule_event driver action
;;

let row_text output =
  (list output).children
  |> List.map ~f:(fun row ->
    let row = View.Expert.describe row in
    let child = View.Expert.describe (List.hd_exn row.children) in
    Gpuio.Key.to_string (Option.value_exn row.key), child.text)
;;

let%expect_test
    "managed component bounds rows, preserves data and handles pins and deletion"
  =
  let data =
    B.Expert.Var.create (source (List.init 20 ~f:(fun i -> i, Int.to_string i)))
  in
  let activations = ref 0 in
  let deactivations = ref 0 in
  let driver =
    create (fun graph ->
      V.component
        (module Int)
        (B.Expert.Var.value data)
        ~row_key:key
        ~config
        ~render_row:(fun ~key:_ ~data ~lifetime:_ graph ->
          B.Edge.lifecycle
            ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr activations)))
            ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr deactivations)))
            graph;
          B.map data ~f:View.text)
        graph)
  in
  assert (V.Output.active_rows (result driver) = 0);
  display driver;
  observe driver [ 0; 1; 2; 3; 4; 5 ];
  let first = result driver in
  assert (V.Output.active_rows first = 4);
  assert (V.Output.budget_exhausted first);
  display driver;
  assert (!activations = 4);
  observe driver ~pins:[ 0 ] [ 10; 11; 12; 13 ];
  let next = result driver in
  assert (V.Output.active_rows next = 4);
  assert (
    List.equal
      [%equal: string * string]
      (row_text next)
      [ "0", "0"; "10", "10"; "11", "11"; "12", "12" ]);
  display driver;
  assert (!activations = 7 && !deactivations = 3);
  let previous = B.Expert.Var.get data in
  B.Expert.Var.set data (C.set previous ~key:0 ~data:"persisted edit" |> Or_error.ok_exn);
  let changed = result driver in
  assert (List.equal Gpuio.Key.equal (payload changed).invalidated [ key 0 ]);
  display driver;
  observe driver [ 18; 19 ];
  ignore (result driver : int V.Output.t);
  display driver;
  observe driver [ 0 ];
  let revisited = result driver in
  assert (
    List.equal [%equal: string * string] (row_text revisited) [ "0", "persisted edit" ]);
  display driver;
  let previous = B.Expert.Var.get data in
  B.Expert.Var.set data (C.splice previous ~at:0 ~remove:1 [] |> Or_error.ok_exn);
  assert (V.Output.active_rows (result driver) = 0);
  display driver;
  assert (!activations = !deactivations);
  assert (C.length (B.Expert.Var.get data) = 19);
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| |}]
;;

let%expect_test "coalesced source changes invalidate from the last accepted baseline" =
  let data = B.Expert.Var.create (source [ 0, "a"; 1, "b"; 2, "c" ]) in
  let driver =
    create (fun graph ->
      V.component
        (module Int)
        (B.Expert.Var.value data)
        ~row_key:key
        ~config
        ~render_row:(fun ~key:_ ~data ~lifetime:_ _ -> B.map data ~f:View.text)
        graph)
  in
  ignore (result driver : int V.Output.t);
  display driver;
  let update key data_ =
    B.Expert.Var.set
      data
      (C.set (B.Expert.Var.get data) ~key ~data:data_ |> Or_error.ok_exn)
  in
  update 0 "first";
  let candidate = result driver in
  let revision = (payload candidate).invalidation_revision in
  update 1 "second";
  let coalesced = result driver in
  assert (Int64.equal (payload coalesced).invalidation_revision revision);
  assert (
    List.equal
      Gpuio.Key.equal
      (List.sort (payload coalesced).invalidated ~compare:Gpuio.Key.compare)
      [ key 0; key 1 ]);
  display driver;
  assert (List.is_empty (payload (result driver)).invalidated);
  update 2 "third";
  let next = result driver in
  assert (Int64.((payload next).invalidation_revision > revision));
  assert (List.equal Gpuio.Key.equal (payload next).invalidated [ key 2 ]);
  display driver;
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| |}]
;;

let%expect_test
    "generation replacement resets transient state and rejects an old controller"
  =
  let generation = B.Expert.Var.create 0L in
  let data = B.return (source [ 0, "record" ]) in
  let driver =
    create (fun graph ->
      V.component
        (module Int)
        data
        ~row_key:key
        ~config
        ~generation:(B.Expert.Var.value generation)
        ~render_row:(fun ~key:_ ~data:_ ~lifetime:_ graph ->
          let count, bump =
            B.state_machine0
              ~default_model:0
              ~apply_action:(fun _ count () -> count + 1)
              graph
          in
          let open B.Let_syntax in
          let%arr count = count
          and bump = bump in
          View.button ~on_click:bump (Int.to_string count))
        graph)
  in
  ignore (result driver : int V.Output.t);
  display driver;
  observe driver [ 0 ];
  let first = result driver in
  display driver;
  let old_controller = V.Output.controller first in
  let row = (list first).children |> List.hd_exn |> View.Expert.describe in
  let button = View.Expert.describe (List.hd_exn row.children) in
  Bonsai_driver.schedule_event driver (Option.value_exn button.on_click ());
  assert (List.equal [%equal: string * string] (row_text (result driver)) [ "0", "1" ]);
  display driver;
  B.Expert.Var.set generation 1L;
  assert (V.Output.active_rows (result driver) = 0);
  display driver;
  Bonsai_driver.schedule_event driver (V.Controller.jump_to_latest old_controller);
  assert (Option.is_none (payload (result driver)).scroll);
  observe driver [ 0 ];
  let next = result driver in
  assert (List.equal [%equal: string * string] (row_text next) [ "0", "0" ]);
  display driver;
  let controller = V.Output.controller next in
  assert (Or_error.is_error (V.Controller.scroll_to controller ~offset:Float.nan 0));
  Bonsai_driver.schedule_event driver (V.Controller.reveal controller 99);
  assert (Option.is_none (payload (result driver)).scroll);
  Bonsai_driver.schedule_event driver (V.Controller.jump_to_latest controller);
  let scroll = (payload (result driver)).scroll |> Option.value_exn in
  let wire =
    Gpuio.Virtual_list.Expert.scroll_to_wire scroll ~find_id:(fun _ -> Some 1L)
    |> Or_error.ok_exn
  in
  assert (Int64.equal wire.serial 1L);
  display driver;
  Bonsai_driver.Expert.invalidate_observers driver;
  [%expect {| |}]
;;

let%expect_test "source key collisions and excessive application pins are explicit errors"
  =
  let collision =
    create (fun graph ->
      V.component
        (module Int)
        (B.return (source [ 0, "a"; 1, "b" ]))
        ~row_key:(fun _ -> key 0)
        ~config
        ~render_row:(fun ~key:_ ~data ~lifetime:_ _ -> B.map data ~f:View.text)
        graph)
  in
  Bonsai_driver.flush collision;
  assert (Or_error.is_error (Bonsai_driver.result collision));
  Bonsai_driver.Expert.invalidate_observers collision;
  let pins =
    create (fun graph ->
      V.component
        (module Int)
        (B.return (source (List.init 5 ~f:(fun i -> i, "row"))))
        ~row_key:key
        ~config
        ~pinned:(B.return [ 0; 1; 2; 3; 4 ])
        ~render_row:(fun ~key:_ ~data ~lifetime:_ _ -> B.map data ~f:View.text)
        graph)
  in
  Bonsai_driver.flush pins;
  assert (Or_error.is_error (Bonsai_driver.result pins));
  Bonsai_driver.Expert.invalidate_observers pins;
  [%expect {| |}]
;;

let%expect_test "full-history visit and revisit release non-default row payloads" =
  let count = 100_000 in
  let page_size = 100 in
  let weak = Stdlib.Weak.create count in
  let data = source (List.init count ~f:(fun i -> i, Int.to_string i)) in
  let config =
    V.Config.create ~max_active:page_size ~height:(Estimated 80.) () |> Or_error.ok_exn
  in
  let allocations = ref 0 in
  let driver =
    create (fun graph ->
      V.component
        (module Int)
        (B.return data)
        ~row_key:key
        ~config
        ~render_row:(fun ~key ~data:_ ~lifetime:_ graph ->
          let payload, set_payload = B.state_opt graph in
          let open B.Let_syntax in
          let on_activate =
            let%arr key = key
            and set_payload = set_payload in
            let open E.Let_syntax in
            let%bind payload =
              E.of_thunk (fun () ->
                Int.incr allocations;
                let payload =
                  String.init 2048 ~f:(fun index ->
                    Char.of_int_exn (32 + ((index + key) mod 90)))
                in
                Stdlib.Weak.set weak key (Some payload);
                payload)
            in
            set_payload (Some payload)
          in
          B.Edge.lifecycle ~on_activate graph;
          let%arr payload = payload in
          View.text
            (Option.value_map payload ~default:"cold" ~f:(fun value ->
               Int.to_string (String.length value))))
        graph)
  in
  ignore (result driver : int V.Output.t);
  display driver;
  Gc.full_major ();
  let baseline = (Gc.stat ()).live_words in
  let live () = List.count (List.init count ~f:Fn.id) ~f:(Stdlib.Weak.check weak) in
  for _pass = 1 to 2 do
    for visit = 0 to (count / page_size) - 1 do
      let keys = List.init page_size ~f:(fun offset -> (visit * page_size) + offset) in
      observe driver keys;
      assert (V.Output.active_rows (result driver) = page_size);
      display driver;
      if visit mod 20 = 19
      then (
        Gc.full_major ();
        assert (live () <= page_size))
    done
  done;
  observe driver [ 0; 1; 2 ];
  ignore (result driver : int V.Output.t);
  display driver;
  assert (!allocations = (2 * count) + 3);
  observe driver [];
  ignore (result driver : int V.Output.t);
  display driver;
  Gc.full_major ();
  assert (live () = 0);
  let growth = (Gc.stat ()).live_words - baseline in
  assert (growth < 150_000);
  assert (C.length data = count && String.equal (C.find data 0 |> Option.value_exn) "0");
  Bonsai_driver.Expert.invalidate_observers driver;
  print_endline
    "100k rows visited twice; zero retained row payloads; retained heap growth below \
     150k words";
  [%expect
    {| 100k rows visited twice; zero retained row payloads; retained heap growth below 150k words |}]
;;

let%expect_test
    "both action-history policies preserve dependent actions within and across batches"
  =
  let run action_history =
    let driver =
      Bonsai_driver.create
        ~action_history
        ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
        (fun graph ->
           let count, add =
             B.state_machine0
               ~default_model:0
               ~apply_action:(fun _ count amount -> count + amount)
               graph
           in
           let observed, capture =
             B.state_machine1
               ~default_model:0
               ~apply_action:(fun _ input old () ->
                 match input with
                 | Bonsai.Computation_status.Active count -> count
                 | Inactive -> old)
               count
               graph
           in
           let open B.Let_syntax in
           let%arr count = count
           and observed = observed
           and add = add
           and capture = capture in
           count, observed, add, capture)
    in
    Bonsai_driver.flush driver;
    for batch = 1 to 100 do
      let _, _, add, capture = Bonsai_driver.result driver in
      Bonsai_driver.schedule_event
        driver
        (E.Many [ add 1; capture (); add 2; capture () ]);
      Bonsai_driver.flush driver;
      let count, observed, _, _ = Bonsai_driver.result driver in
      assert (count = 3 * batch && observed = count)
    done;
    Bonsai_driver.Expert.invalidate_observers driver
  in
  run Keep_recent;
  run Release_after_flush;
  [%expect {| |}]
;;

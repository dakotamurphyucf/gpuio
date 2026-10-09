open Core
open Gpuio
module W = Gpuio_protocol.Wire
module LI = Gpuio_protocol.List_input_wire
module R = Reconciler

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let config = Virtual_list.Config.create ~height:(Fixed 24.) () |> ok

let input
      ?(epoch = "source")
      ?(cursor = "a")
      ?(query = Some "query")
      ?(disabled = false)
      ?(busy = false)
      ?(select = false)
      ()
  =
  List_input.Config.create
    ~epoch:(key epoch)
    ~cursor:(key cursor)
    ?query:(Option.map query ~f:key)
    ~disabled
    ~busy
    ~selection_on_navigation:select
    ()
;;

let list ?(name = "list") ?(multiple = true) ?(enabled = true) ?(input = input ()) keys =
  let rows =
    List.mapi keys ~f:(fun index name ->
      let item =
        Accessibility.Option_item.create ~index ~count:(List.length keys) () |> ok
      in
      let a = Accessibility.create ~role:(Option_item item) ~label:name () |> ok in
      key name, View.with_accessibility (View.column [ View.text name ]) a |> ok)
  in
  let view =
    View.virtual_list ~key:(key name) ~config ~on_viewport:(fun _ -> None) rows |> ok
  in
  let view =
    View.with_accessibility
      view
      (Accessibility.create ~role:(List_box multiple) ~label:"Results" () |> ok)
    |> ok
  in
  if enabled
  then View.with_list_input view ~config:input ~on_input:Option.some |> ok
  else view
;;

let query ?(controller = "query") ?(mode = Text_input.Mode.Single_line) () =
  View.text_input
    ~controller:(key controller)
    ~config:(Text_input.Config.create ~mode ~label:"Search" () |> ok)
    ~on_event:(fun _ -> None)
    ()
  |> ok
  |> fun v -> View.with_key v (key "query")
;;

let frame ?(first = true) ?(query = query ()) list =
  View.column (if first then [ query; list ] else [ list; query ])
;;

let prepare r view = R.prepare r ~theme:Theme.default (Some view) |> ok

let ops u =
  match R.message u with
  | Some (Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let accept r u = R.accept r u |> ok

let wire u =
  List.find_map_exn (ops u) ~f:(function
    | W.Op.Set_list_input (node, Some config) -> Some (node, config)
    | _ -> None)
;;

let owner u =
  List.find_map_exn (ops u) ~f:(function
    | W.Op.Create (node, Virtual_list, _, Some handler) -> Some (node, handler)
    | _ -> None)
;;

let dispatch r (node, handler) ?(generation = 1L) ?(revision = 1L) request =
  R.dispatch r (W.Event.List_input (window, node, handler, revision, generation, request))
  |> Option.join
;;

let has_input u =
  List.exists (ops u) ~f:(function
    | W.Op.Set_list_input _ -> true
    | _ -> false)
;;

let%expect_test
    "all requests map keys without inventing state or accepting invalid targets"
  =
  let requests =
    LI.Request.
      [ Navigate (Previous, None)
      ; Navigate (Next, Some Replace)
      ; Navigate (First, Some Toggle)
      ; Navigate (Last, Some (Range { extend = true }))
      ; Select (1L, Replace)
      ; Focus 1L
      ; Select_active Toggle
      ; Confirm (1L, Primary)
      ; Confirm_active Secondary
      ; Context 1L
      ; Context_active
      ; Set_selected (1L, false)
      ; Cancel
      ]
  in
  List.iter requests ~f:(fun request ->
    let mapped =
      List_input.Expert.of_wire request ~find_key:(fun id ->
        if Int64.equal id 1L then Some "a" else None)
    in
    assert (Option.is_some mapped));
  List.iter
    LI.Request.
      [ Select (0L, Replace)
      ; Focus (-1L)
      ; Confirm (99L, Secondary)
      ; Context 99L
      ; Set_selected (99L, true)
      ]
    ~f:(fun request ->
      assert (Option.is_none (List_input.Expert.of_wire request ~find_key:(fun _ -> None))));
  print_s
    [%sexp
      (List_input.Expert.of_wire (Confirm (1L, Secondary)) ~find_key:(fun _ -> Some "a")
       : string List_input.t option)];
  [%expect {| ((Confirm a Secondary)) |}]
;;

let%expect_test
    "either sibling order resolves query, retains input on reorder and skips unchanged \
     frames"
  =
  List.iter [ true; false ] ~f:(fun first ->
    let r = R.create window in
    let l = list [ "a"; "b" ] in
    let q = query () in
    let initial = prepare r (frame ~first ~query:q l) in
    let _, cfg = wire initial in
    let query_id =
      List.find_map_exn (ops initial) ~f:(function
        | W.Op.Create (n, Input, _, _) -> Some n
        | _ -> None)
    in
    assert (Option.equal Gpuio_protocol.Node_id.equal cfg.query (Some query_id));
    assert (Option.equal Int64.equal cfg.cursor (Some 1L));
    assert (Int64.equal cfg.generation 1L);
    accept r initial;
    let view = frame ~first:(not first) ~query:q l in
    let reorder = prepare r view in
    assert (not (has_input reorder));
    accept r reorder;
    let unchanged = prepare r view in
    assert (Option.is_none (R.message unchanged));
    accept r unchanged;
    assert (Option.is_some (dispatch r (owner initial) (Confirm_active Primary)));
    R.close r);
  [%expect {| |}]
;;

let%expect_test
    "cursor and busy echoes preserve ordered relative requests; policy and speculative \
     epochs do not"
  =
  let r = R.create window in
  let initial = prepare r (frame (list [ "a"; "b" ])) in
  let route = owner initial in
  accept r initial;
  let cursor =
    prepare r (frame (list ~input:(input ~cursor:"b" ~busy:true ()) [ "a"; "b" ]))
  in
  let _, cfg = wire cursor in
  assert (Int64.equal cfg.generation 1L && cfg.busy);
  accept r cursor;
  List.iter
    LI.Request.
      [ Navigate (Next, None)
      ; Navigate (Previous, None)
      ; Confirm_active Primary
      ; Context_active
      ; Cancel
      ]
    ~f:(fun request -> assert (Option.is_some (dispatch r route request)));
  let abandoned =
    prepare r (frame (list ~input:(input ~epoch:"abandoned" ()) [ "a"; "b" ]))
  in
  let changed =
    prepare r (frame (list ~input:(input ~epoch:"replacement" ()) [ "a"; "b" ]))
  in
  assert (Int64.equal (snd (wire abandoned)).generation 2L);
  assert (Int64.equal (snd (wire changed)).generation 2L);
  assert (Option.is_some (dispatch r route Cancel));
  accept r changed;
  assert (Result.is_error (R.accept r abandoned));
  assert (Option.is_none (dispatch r route Cancel));
  assert (Option.is_some (dispatch r route ~generation:2L Cancel));
  let disabled =
    prepare
      r
      (frame (list ~input:(input ~epoch:"replacement" ~disabled:true ()) [ "a"; "b" ]))
  in
  assert (Int64.equal (snd (wire disabled)).generation 3L);
  accept r disabled;
  assert (Option.is_none (dispatch r route ~generation:3L Cancel));
  let enabled =
    prepare r (frame (list ~input:(input ~epoch:"replacement" ()) [ "a"; "b" ]))
  in
  assert (Int64.equal (snd (wire enabled)).generation 4L);
  accept r enabled;
  let clear = prepare r (frame (list ~enabled:false [ "a"; "b" ])) in
  assert (
    List.exists (ops clear) ~f:(function
      | Set_list_input (_, None) -> true
      | _ -> false));
  accept r clear;
  assert (Option.is_none (dispatch r route ~generation:4L Cancel));
  let reinstall =
    prepare r (frame (list ~input:(input ~epoch:"replacement" ()) [ "a"; "b" ]))
  in
  assert (Int64.equal (snd (wire reinstall)).generation 5L);
  accept r reinstall;
  assert (Option.is_none (dispatch r route ~generation:4L Cancel));
  assert (Option.is_some (dispatch r route ~generation:5L Cancel));
  let policy =
    prepare
      r
      (frame (list ~input:(input ~epoch:"replacement" ~select:true ()) [ "a"; "b" ]))
  in
  assert (Int64.equal (snd (wire policy)).generation 6L);
  accept r policy;
  let mode =
    prepare
      r
      (frame
         (list
            ~multiple:false
            ~input:(input ~epoch:"replacement" ~select:true ())
            [ "a"; "b" ]))
  in
  assert (Int64.equal (snd (wire mode)).generation 7L);
  accept r mode;
  R.close r;
  [%expect {| |}]
;;

let%expect_test
    "query edits revalidate even a physically shared list; invalid relationships roll \
     back"
  =
  let r = R.create window in
  let l = list [ "a"; "b" ] in
  let initial = prepare r (frame l) in
  let route = owner initial in
  accept r initial;
  let invalids =
    [ View.column [ l ]
    ; frame ~query:(View.text ~key:(key "query") "not an input") l
    ; frame ~query:(query ~mode:Multiline ()) l
    ; View.column [ View.column [ query () ]; l ]
    ; View.column [ query (); l; list ~name:"other" [ "a"; "b" ] ]
    ]
  in
  List.iter invalids ~f:(fun view ->
    assert (Result.is_error (R.prepare r ~theme:Theme.default (Some view))));
  assert (Option.is_some (dispatch r route Cancel));
  let rebound =
    prepare r (frame ~first:false ~query:(query ~controller:"new controller" ()) l)
  in
  assert (Int64.equal (snd (wire rebound)).generation 2L);
  accept r rebound;
  assert (Option.is_none (dispatch r route Cancel));
  assert (Option.is_some (dispatch r route ~generation:2L Cancel));
  R.close r;
  let root = R.create window in
  assert (Result.is_error (R.prepare root ~theme:Theme.default (Some l)));
  let no_query = list ~input:(input ~query:None ()) [ "a"; "b" ] in
  let mounted = prepare root no_query in
  assert (Option.is_none (snd (wire mounted)).query);
  accept root mounted;
  R.close root;
  [%expect {| |}]
;;

let%expect_test
    "removed keys retire targets, old revisions can navigate, future and foreign events \
     cannot"
  =
  let r = R.create window in
  let initial = prepare r (frame (list [ "a"; "b" ])) in
  let node, handler = owner initial in
  accept r initial;
  let route = node, handler in
  assert (Option.is_some (dispatch r route (Context 2L)));
  accept r (prepare r (frame (list [ "a" ])));
  assert (Option.is_none (dispatch r route (Context 2L)));
  accept r (prepare r (frame (list [ "a"; "b" ])));
  assert (Option.is_none (dispatch r route (Confirm (2L, Primary))));
  (match dispatch r route (Set_selected (3L, true)) with
   | Some (List_input.Set_selected (k, true)) -> assert (Key.equal k (key "b"))
   | _ -> assert false);
  assert (Option.is_some (dispatch r route (Navigate (Next, None))));
  assert (Option.is_none (dispatch r route ~revision:(Int64.succ (R.revision r)) Cancel));
  assert (Option.is_none (dispatch r route ~revision:(-1L) Cancel));
  let wrong_handler = Gpuio_protocol.Handler_id.create ~slot:99L ~generation:1L |> ok in
  assert (Option.is_none (dispatch r (node, wrong_handler) Cancel));
  let foreign = Gpuio_protocol.Window_id.create ~slot:99L ~generation:1L |> ok in
  assert (
    Option.is_none
      (R.dispatch r (W.Event.List_input (foreign, node, handler, 1L, 1L, Cancel))));
  R.close r;
  assert (Option.is_none (dispatch r route Cancel));
  [%expect {| |}]
;;

let%expect_test
    "100k logical rows update the cursor without rematerializing order or rows"
  =
  let order =
    Virtual_list.Order.create (List.init 100_000 ~f:(fun i -> key (Int.to_string i)))
    |> ok
  in
  let config = Virtual_list.Config.create ~max_active:1 ~height:(Fixed 24.) () |> ok in
  let base =
    View.Expert.managed_virtual_list
      ~config
      ~order
      ~on_viewport:(fun _ -> None)
      ~on_retain:(fun _ -> None)
      []
    |> ok
  in
  let base =
    View.with_accessibility
      base
      (Accessibility.create ~role:(List_box false) ~label:"Large" () |> ok)
    |> ok
  in
  let view cursor =
    View.with_list_input
      base
      ~config:
        (List_input.Config.create
           ~epoch:(key "large")
           ~cursor:(key (Int.to_string cursor))
           ())
      ~on_input:Option.some
    |> ok
  in
  let r = R.create window in
  let initial = prepare r (view 0) in
  accept r initial;
  List.iter [ 99_999; 50_000; 0 ] ~f:(fun cursor ->
    let u = prepare r (view cursor) in
    (match ops u with
     | [ Set_list_input (_, Some cfg) ] -> assert (Int64.equal cfg.generation 1L)
     | _ -> assert false);
    accept r u);
  R.close r;
  [%expect {| |}]
;;

let%expect_test
    "late semantics edits and disabled mounted cursors fail during preparation"
  =
  let r = R.create window in
  let invalid =
    list ~input:(input ~query:None ()) [ "a" ]
    |> fun v -> View.with_accessibility v (Accessibility.create ~role:Log () |> ok) |> ok
  in
  assert (Result.is_error (R.prepare r ~theme:Theme.default (Some invalid)));
  let item = Accessibility.Option_item.create ~index:0 ~disabled:true () |> ok in
  let row =
    View.with_accessibility
      (View.column [])
      (Accessibility.create ~role:(Option_item item) () |> ok)
    |> ok
  in
  let base =
    View.virtual_list ~config [ key "a", row ]
    |> ok
    |> fun v ->
    View.with_accessibility v (Accessibility.create ~role:(List_box false) () |> ok) |> ok
  in
  let invalid =
    View.with_list_input base ~config:(input ~query:None ()) ~on_input:Option.some |> ok
  in
  assert (Result.is_error (R.prepare r ~theme:Theme.default (Some invalid)));
  assert (Int64.equal (R.revision r) 0L);
  R.close r;
  [%expect {| |}]
;;

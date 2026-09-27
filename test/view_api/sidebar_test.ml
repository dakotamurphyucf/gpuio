open Core
open Gpuio
module S = Sidebar
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let id text = S.Id.of_string text |> ok

let item ?(disabled = false) ?(children = []) label =
  S.Item.create ~id:(id label) ~label ~disabled ~children () |> ok
;;

let group items = S.Group.create ~id:(id "main") ~label:"Main" items |> ok

let model () =
  S.create
    ~groups:
      [ group
          [ item ~children:[ item "Inbox"; item ~disabled:true "Locked" ] "Archive"
          ; item "Settings"
          ]
      ]
    ~selected:(Some (id "Inbox"))
    ~expanded:[ id "Archive" ]
    ()
  |> ok
;;

let%expect_test "selection, expansion, collapse and latest eligibility remain independent"
  =
  let initial = model () in
  let collapsed = S.apply_request initial (Toggle (id "Archive")) in
  assert (Option.equal S.Id.equal (S.selected collapsed) (Some (id "Inbox")));
  assert (not (S.is_visible collapsed (id "Inbox")));
  assert (S.equal collapsed (S.apply_request collapsed (Select (id "Inbox"))));
  let reopened = S.apply_request collapsed (Toggle (id "Archive")) in
  assert (S.equal initial reopened);
  assert (S.equal initial (S.apply_request initial (Select (id "Locked"))));
  assert (S.equal initial (S.apply_request initial (Toggle (id "Settings"))));
  let selected = S.apply_request initial (Select (id "Settings")) in
  assert (S.is_expanded selected (id "Archive"));
  let compact = S.apply_request selected Toggle_collapsed in
  assert (S.is_visible compact (id "Archive") && not (S.is_visible compact (id "Inbox")));
  assert (S.equal compact (S.apply_request compact (Toggle (id "Archive"))));
  assert (S.is_expanded compact (id "Archive"));
  let offcanvas = S.with_collapse compact Offcanvas in
  assert (not (S.is_visible offcanvas (id "Archive")));
  let fixed = S.with_collapse offcanvas Never in
  assert (not (S.is_collapsed fixed));
  assert (S.is_visible fixed (id "Inbox"));
  assert (S.equal fixed (S.apply_request fixed Toggle_collapsed));
  let disabled = S.with_disabled initial true in
  assert (S.equal disabled (S.apply_request disabled Toggle_collapsed));
  assert (S.equal disabled (S.apply_request disabled (Select (id "Settings"))));
  assert (
    Option.equal
      S.Id.equal
      (S.selected (S.select disabled (Some (id "Locked")) |> ok))
      (Some (id "Locked")));
  print_endline
    "hidden/disabled/stale requests are inert; selected route and expanded branches \
     survive collapse modes";
  [%expect
    {| hidden/disabled/stale requests are inert; selected route and expanded branches survive collapse modes |}]
;;

let%expect_test "collection replacement normalizes history without choosing another route"
  =
  let original = model () in
  let next = S.with_groups original [ group [ item "Archive"; item "Settings" ] ] |> ok in
  assert (Option.is_none (S.selected next));
  assert (List.is_empty (S.expanded next));
  assert (S.equal next (S.apply_request next (Select (id "Inbox"))));
  assert (Result.is_error (S.select next (Some (id "Absent"))));
  List.iter
    [ [ group [ item "Duplicate"; item "Duplicate" ] ]
    ; [ group [ item ~children:[ item "Shared" ] "Parent"; item "Shared" ] ]
    ; [ group []; group [] ]
    ]
    ~f:(fun groups -> assert (Result.is_error (S.create ~groups ~selected:None ())));
  List.iter
    [ [ id "Missing" ]; [ id "Settings" ]; [ id "Archive"; id "Archive" ] ]
    ~f:(fun expanded ->
      assert (
        Result.is_error (S.create ~groups:(S.groups original) ~selected:None ~expanded ())));
  print_endline
    "removed selection clears; absent/leaf expansions drop; duplicates and invalid \
     construction reject";
  [%expect
    {| removed selection clears; absent/leaf expansions drop; duplicates and invalid construction reject |}]
;;

let%expect_test "item, group, depth and retained-text budgets are enforced" =
  let items = List.init S.max_items ~f:(fun i -> item (Int.to_string i)) in
  let maximum = S.create ~groups:[ group items ] ~selected:None () |> ok in
  assert (Option.is_some (S.find maximum (id "4095")));
  assert (Result.is_error (S.Group.create ~id:(id "too-many") (item "Extra" :: items)));
  let groups =
    List.init S.max_groups ~f:(fun i ->
      S.Group.create ~id:(id (Int.to_string i)) [] |> ok)
  in
  ignore (S.create ~groups ~selected:None () |> ok : S.t);
  assert (Result.is_error (S.create ~groups:(group [] :: groups) ~selected:None ()));
  let deep =
    List.fold (List.range 1 S.max_depth) ~init:(item "0") ~f:(fun child depth ->
      item ~children:[ child ] (Int.to_string depth))
  in
  assert (
    Result.is_error
      (S.Item.create ~id:(id "overflow") ~label:"Overflow" ~children:[ deep ] ()));
  let large =
    List.init 64 ~f:(fun i ->
      S.Item.create ~id:(id (Int.to_string i)) ~label:(String.make 4096 'x') () |> ok)
  in
  assert (Result.is_error (S.Group.create ~id:(id "text-overflow") large));
  List.iter
    [ ""; "\255"; "x\000y"; String.make 4097 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (S.Item.create ~id:(id "test") ~label ()));
      assert (
        Result.is_error
          (S.Item.create ~id:(id "test") ~label:"Valid" ~compact_label:label ())));
  print_endline
    "4096 items, 128 groups and depth 16 accepted; overflow and malformed text reject";
  [%expect
    {| 4096 items, 128 groups and depth 16 accepted; overflow and malformed text reject |}]
;;

let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let commit reconciler view =
  let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept reconciler update |> ok;
  match Reconciler.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let sidebar ?(hidden = Content_policy.Retain) model =
  S.view model ~hidden ~on_request:Fn.id () |> ok
;;

let%expect_test "width motion is native, bounded and stable across target changes" =
  List.iter [ -1.; 10_001. ] ~f:(fun ms ->
    assert (Result.is_error (S.Motion.create ~duration:(Time_ns.Span.of_ms ms) ())));
  ignore (S.Motion.create ~duration:(Time_ns.Span.of_sec 10.) () |> ok : S.Motion.t);
  let reconciler = Reconciler.create window in
  let model = model () in
  let first = commit reconciler (sidebar model) in
  let owner, config =
    List.find_map_exn first ~f:(function
      | W.Op.Set_animation (node, config) -> Some (node, config)
      | _ -> None)
  in
  assert (Option.is_none config.initial);
  assert (Int64.equal config.duration_ms 200L);
  assert (List.is_empty (commit reconciler (sidebar model)));
  List.iter
    [ S.with_collapsed model true, 56.
    ; S.with_collapse (S.with_collapsed model true) Offcanvas, 0.
    ; model, 240.
    ]
    ~f:(fun (model, width) ->
      let changes = commit reconciler (sidebar model) in
      assert (
        not
          (List.exists changes ~f:(function
             | W.Op.Remove _ -> true
             | _ -> false)));
      let config =
        List.find_map_exn changes ~f:(function
          | W.Op.Set_animation (node, config) when Gpuio_protocol.Node_id.equal owner node
            -> Some config
          | _ -> None)
      in
      assert (Option.is_none config.initial);
      assert (
        W.Animation.Target.equal
          (List.hd_exn config.targets)
          { property = Width; value = width }));
  let appearance = S.Appearance.create ~motion:S.Motion.immediate () |> ok in
  let changes =
    commit reconciler (S.view model ~appearance ~hidden:Retain ~on_request:Fn.id () |> ok)
  in
  assert (
    List.exists changes ~f:(function
      | W.Op.Set_animation (node, config) ->
        Gpuio_protocol.Node_id.equal node owner && Int64.equal config.duration_ms 0L
      | _ -> false));
  print_endline
    "stable native owner; initial placement settles; target widths 56/0/240; immediate \
     policy does not remount";
  [%expect
    {| stable native owner; initial placement settles; target widths 56/0/240; immediate policy does not remount |}]
;;

let button operations label =
  List.find_map_exn operations ~f:(function
    | W.Op.Create (node, Button, text, Some handler) when String.equal text label ->
      Some (node, handler)
    | _ -> None)
;;

let%expect_test
    "icon and offcanvas updates preserve retained links, fence hidden intent and unmount \
     explicitly"
  =
  let initial = model () in
  let reconciler = Reconciler.create window in
  let mounted = commit reconciler (sidebar initial) in
  let parent, parent_handler = button mounted "Archive" in
  let child, child_handler = button mounted "Inbox" in
  let compact = S.with_collapsed initial true in
  let changes = commit reconciler (sidebar compact) in
  assert (
    not
      (List.exists changes ~f:(function
         | W.Op.Remove n ->
           Gpuio_protocol.Node_id.equal n parent || Gpuio_protocol.Node_id.equal n child
         | _ -> false)));
  let request =
    Reconciler.dispatch reconciler (W.Event.Press (window, parent, parent_handler, 1L))
    |> Option.value_exn
  in
  assert (S.Request.equal request (Select (id "Archive")));
  let queued =
    Reconciler.dispatch reconciler (W.Event.Press (window, child, child_handler, 1L))
    |> Option.value_exn
  in
  assert (S.equal compact (S.apply_request compact queued));
  let offcanvas = S.with_collapse compact Offcanvas in
  let hidden = commit reconciler (sidebar offcanvas) in
  assert (
    not
      (List.exists hidden ~f:(function
         | W.Op.Remove _ -> true
         | _ -> false)));
  let removed = commit reconciler (sidebar ~hidden:Unmount offcanvas) in
  assert (
    List.exists removed ~f:(function
      | W.Op.Remove n -> Gpuio_protocol.Node_id.equal n child
      | _ -> false));
  assert (
    Option.is_none
      (Reconciler.dispatch reconciler (W.Event.Press (window, child, child_handler, 1L))));
  print_endline
    "compact/offcanvas retain identity; latest model rejects hidden requests; Unmount \
     retires handlers";
  [%expect
    {| compact/offcanvas retain identity; latest model rejects hidden requests; Unmount retires handlers |}]
;;

let%expect_test
    "hidden unmounted callbacks are skipped; dynamic labels and widths validate"
  =
  let built = ref 0 in
  let t = model () |> fun t -> S.with_collapsed (S.with_collapse t Offcanvas) true in
  ignore
    (S.view
       t
       ~hidden:Unmount
       ~on_request:Fn.id
       ~decorate:(fun _ ->
         incr built;
         S.Decoration.create ())
       ~header:(fun ~compact:_ ->
         incr built;
         View.text "Header")
       ()
     |> ok
     : _ View.t);
  assert (!built = 0);
  let labels =
    S.Labels.create
      ~navigation:"Sidebar"
      ~current:"Current"
      ~expand_sidebar:"Expand"
      ~collapse_sidebar:"Collapse"
      ~toggle_item:(fun ~label:_ ~expanded:_ -> "")
    |> ok
  in
  assert (Result.is_error (S.view (model ()) ~labels ~hidden:Retain ~on_request:Fn.id ()));
  List.iter
    [ Float.nan, 56.; 240., Float.infinity; 20., 16.; 240., 300.; 4097., 56. ]
    ~f:(fun (width, compact_width) ->
      assert (Result.is_error (S.Appearance.create ~width ~compact_width ())));
  print_endline
    "unmounted content construction skipped; invalid localization and geometry return \
     errors";
  [%expect
    {| unmounted content construction skipped; invalid localization and geometry return errors |}]
;;

let%expect_test
    "custom disclosure header preserves independent actions and validates its toggle"
  =
  let make trigger =
    View.disclosure_with_header
      ~label:"Children"
      ~expanded:true
      ~hidden:Retain
      ~header:[ View.button ~on_click:(fun () -> `Navigate) "Parent" ]
      ~trigger
      [ View.text "Body" ]
  in
  assert (Result.is_error (make (View.text "Not a button")));
  let tree = make (View.button ~on_click:(fun () -> `Toggle) "+") |> ok in
  let reconciler = Reconciler.create window in
  let operations = commit reconciler tree in
  List.iter
    [ "Parent", `Navigate; "+", `Toggle ]
    ~f:(fun (label, expected) ->
      let node, handler = button operations label in
      assert (
        Option.equal
          [%equal: [ `Navigate | `Toggle ]]
          (Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L)))
          (Some expected)));
  print_endline
    "header navigation and dedicated expansion dispatch independently; nonbutton trigger \
     rejected";
  [%expect
    {| header navigation and dedicated expansion dispatch independently; nonbutton trigger rejected |}]
;;

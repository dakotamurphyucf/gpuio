open Core
open Gpuio
module W = Gpuio_protocol.Wire
module A = Gpuio_protocol.Accessibility_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let describe = View.Expert.describe

let metadata view =
  (describe view).accessibility |> Option.value_exn |> Accessibility.Expert.to_wire
;;

let commit reconciler view =
  let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept reconciler update |> ok;
  match Reconciler.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let button operations label =
  List.find_map_exn operations ~f:(function
    | W.Op.Create (id, Button, text, Some handler) when String.equal label text ->
      Some (id, handler)
    | _ -> None)
;;

let dispatch reconciler (node, handler) =
  Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L))
;;

let page model = Navigation.pagination model ~on_request:Fn.id () |> ok

let%expect_test
    "page count does not determine native tree size; current is distinct from focus"
  =
  List.iter [ 0; 1; 10; Pagination.max_pages ] ~f:(fun count ->
    let model = Pagination.create ~total_pages:count ~siblings:4 () |> ok in
    let view = page model in
    assert (Option.equal A.Role.equal (metadata view).role (Some Navigation));
    let children = (describe view).children in
    assert (List.length children <= 17);
    let marked = List.filter children ~f:(fun v -> Option.is_some (metadata v).current) in
    assert (List.length marked = if count = 0 then 0 else 1);
    List.iter marked ~f:(fun v ->
      assert (Option.equal A.Current.equal (metadata v).current (Some Page));
      assert (Option.equal String.equal (metadata v).description (Some "Current page")));
    let ops = commit (Reconciler.create window) view in
    let handlers =
      List.count ops ~f:(function
        | W.Op.Create (_, _, _, Some _) -> true
        | _ -> false)
    in
    printf "%d pages: %d children, %d handlers\n" count (List.length children) handlers);
  [%expect
    {|
    0 pages: 4 children, 0 handlers
    1 pages: 5 children, 1 handlers
    10 pages: 11 children, 8 handlers
    1000000000 pages: 11 children, 8 handlers
    |}]
;;

let%expect_test
    "queued relative requests use latest state and removed page events cannot navigate"
  =
  let model = Pagination.create ~total_pages:100 ~current:50 () |> ok in
  let reconciler = Reconciler.create window in
  let initial = commit reconciler (page model) in
  let next = button initial "Next" in
  let old_last = button initial "100" in
  let requests = List.init 3 ~f:(fun _ -> dispatch reconciler next |> Option.value_exn) in
  let advanced = List.fold requests ~init:model ~f:Pagination.apply_request in
  assert (Option.equal Int.equal (Pagination.current advanced) (Some 53));
  let shrunk = Pagination.with_total_pages advanced 2 |> ok in
  let updates = commit reconciler (page shrunk) in
  assert (Option.is_none (dispatch reconciler old_last));
  assert (Option.is_none (dispatch reconciler next));
  assert (
    Pagination.equal
      shrunk
      (Pagination.apply_request shrunk (Pagination.Request.page 100 |> ok)));
  let next_id, _ = next in
  assert (
    not
      (List.exists updates ~f:(function
         | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id next_id
         | _ -> false)));
  print_endline
    "3 queued Next requests reach 53; shrink clamps to 2; stale and disabled handlers \
     are fenced";
  [%expect
    {| 3 queued Next requests reach 53; shrink clamps to 2; stale and disabled handlers are fenced |}]
;;

let%expect_test
    "same keyed current page retains button identity and clearing is a metadata update"
  =
  let model = Pagination.create ~total_pages:20 ~current:10 () |> ok in
  let reconciler = Reconciler.create window in
  let initial = commit reconciler (page model) in
  let current_id, _ = button initial "10" in
  let next = Pagination.apply_request model Pagination.Request.next in
  let updates = commit reconciler (page next) in
  assert (
    not
      (List.exists updates ~f:(function
         | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id current_id
         | _ -> false)));
  assert (
    List.exists updates ~f:(function
      | W.Op.Set_accessibility (id, Some config) ->
        Gpuio_protocol.Node_id.equal id current_id && Option.is_none config.current
      | _ -> false));
  let disabled = commit reconciler (page (Pagination.with_disabled next true)) in
  assert (
    not
      (List.exists disabled ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  print_endline
    "current changes preserve overlapping pages; disabling does not replace the tree";
  [%expect
    {| current changes preserve overlapping pages; disabling does not replace the tree |}]
;;

let collection labels =
  List.map labels ~f:(fun (id, label, disabled) ->
    Choice.create ~id:(Choice.Id.of_string id |> ok) ~label ~disabled () |> ok)
  |> Choice.Collection.create
  |> ok
;;

let breadcrumbs items suffix =
  Navigation.breadcrumbs
    items
    ~label:"Workspace"
    ~current_description:"Current location"
    ~on_navigate:(fun id -> Choice.Id.to_string id ^ suffix)
    ()
  |> ok
;;

let%expect_test
    "breadcrumbs preserve route identity across labels and use the latest callback"
  =
  let items =
    collection
      [ "root", "Home", false; "locked", "Private", true; "inbox", "Inbox", false ]
  in
  let view = breadcrumbs items "-first" in
  let root = describe view in
  assert (List.length root.children = 3);
  let current = List.last_exn (describe (List.last_exn root.children)).children in
  assert (Option.equal A.Current.equal (metadata current).current (Some Location));
  let reconciler = Reconciler.create window in
  let initial = commit reconciler view in
  let home = button initial "Home" in
  assert (
    List.count initial ~f:(function
      | W.Op.Create (_, _, _, Some _) -> true
      | _ -> false)
    = 1);
  let changed = collection [ "root", "Accueil", false; "inbox", "Boîte", false ] in
  let updates = commit reconciler (breadcrumbs changed "-latest") in
  assert (Option.equal String.equal (dispatch reconciler home) (Some "root-latest"));
  assert (
    not
      (List.exists updates ~f:(function
         | W.Op.Create (_, Button, _, _) -> true
         | _ -> false)));
  print_endline
    "current has no action; disabled link is inert; route key and latest callback \
     survive localization";
  [%expect
    {| current has no action; disabled link is inert; route key and latest callback survive localization |}]
;;

let%expect_test
    "localized formatters run for bounded visible items and invalid results return errors"
  =
  let pages = ref 0 in
  let gaps = ref 0 in
  let labels page =
    Navigation.Pagination_labels.create
      ~navigation:"Pages"
      ~first:"First"
      ~previous:"Previous"
      ~next:"Next"
      ~last:"Last"
      ~current:"Current page"
      ~page
      ~gap:(fun ~first:_ ~last:_ ->
        incr gaps;
        "Omitted pages")
    |> ok
  in
  let model =
    Pagination.create ~total_pages:Pagination.max_pages ~current:500_000_000 () |> ok
  in
  ignore
    (Navigation.pagination
       model
       ~labels:
         (labels (fun n ->
            incr pages;
            Int.to_string n))
       ~on_request:Fn.id
       ()
     |> ok
     : _ View.t);
  assert (!pages <= 11 && !gaps <= 2);
  List.iter
    [ ""; "\255"; "bad\000text"; String.make 4097 'x' ]
    ~f:(fun bad ->
      assert (
        Result.is_error
          (Navigation.pagination
             model
             ~labels:(labels (fun _ -> bad))
             ~on_request:Fn.id
             ()));
      assert (
        Result.is_error
          (Navigation.breadcrumbs
             (collection [])
             ~label:"Path"
             ~current_description:bad
             ~on_navigate:Fn.id
             ())));
  print_endline
    "formatting is bounded and malformed localized strings fail before reconciliation";
  [%expect
    {| formatting is bounded and malformed localized strings fail before reconciliation |}]
;;

let%expect_test "compact paging retains controls and skips invisible formatters" =
  let model = Pagination.create ~total_pages:100 ~current:50 () |> ok in
  let labels =
    Navigation.Pagination_labels.create
      ~navigation:"Pages"
      ~first:"First"
      ~previous:"Earlier"
      ~next:"Later"
      ~last:"Last"
      ~current:"Current"
      ~page:(fun _ -> failwith "invisible page formatted")
      ~gap:(fun ~first:_ ~last:_ -> failwith "invisible gap formatted")
    |> ok
  in
  let compact model =
    Navigation.pagination model ~layout:Compact ~labels ~on_request:Fn.id () |> ok
  in
  let view = compact model in
  let children = (describe view).children in
  assert (List.length children = 2);
  assert (
    Option.equal String.equal (metadata (List.hd_exn children)).label (Some "Earlier"));
  assert (
    Option.equal String.equal (metadata (List.last_exn children)).label (Some "Later"));
  let reconciler = Reconciler.create window in
  let initial = commit reconciler (page model) in
  let previous = button initial "Previous"
  and next = button initial "Next" in
  let updates = commit reconciler view in
  List.iter [ previous; next ] ~f:(fun (node, _) ->
    assert (
      not
        (List.exists updates ~f:(function
           | W.Op.Remove id -> Gpuio_protocol.Node_id.equal node id
           | _ -> false))));
  assert (
    Option.equal
      Pagination.Request.equal
      (dispatch reconciler next)
      (Some Pagination.Request.next));
  List.iter [ 0; 1 ] ~f:(fun total_pages ->
    ignore
      (commit reconciler (compact (Pagination.create ~total_pages () |> ok))
       : W.Op.t list);
    assert (Option.is_none (dispatch reconciler next)));
  print_endline
    "two labelled arrows; retained identity and request; empty/single page inert; no \
     hidden formatting";
  [%expect
    {| two labelled arrows; retained identity and request; empty/single page inert; no hidden formatting |}]
;;

let%expect_test "interactive gaps are bounded and obsolete intervals lose handlers" =
  let model =
    Pagination.create ~total_pages:Pagination.max_pages ~current:500_000_000 () |> ok
  in
  let render model suffix =
    Navigation.pagination
      model
      ~on_request:(fun _ -> "request")
      ~on_gap:(fun ~first ~last -> sprintf "%d..%d:%s" first last suffix)
      ()
    |> ok
  in
  let reconciler = Reconciler.create window in
  let initial = commit reconciler (render model "old") in
  let gap = button initial "…" in
  assert (List.length (describe (render model "current")).children <= 17);
  ignore (commit reconciler (render model "current") : W.Op.t list);
  print_s [%sexp (dispatch reconciler gap : string option)];
  ignore
    (commit reconciler (render (Pagination.with_disabled model true) "disabled")
     : W.Op.t list);
  assert (Option.is_none (dispatch reconciler gap));
  ignore
    (commit reconciler (render (Pagination.with_total_pages model 3 |> ok) "shrunk")
     : W.Op.t list);
  assert (Option.is_none (dispatch reconciler gap));
  [%expect {| (2..499999998:current) |}]
;;

let%expect_test "passive breadcrumb members replace links without stale activation" =
  let items =
    collection
      [ "root", "Home", false; "section", "Section", false; "here", "Here", false ]
  in
  let render ~passive =
    Navigation.breadcrumbs
      items
      ~label:"Path"
      ~current_description:"Here"
      ~is_navigable:(fun item ->
        not
          (passive
           && Choice.Id.equal (Choice.id item) (Choice.Id.of_string "section" |> ok)))
      ~item_style:(fun item ->
        Style.create_exn
          [ Font_weight
              (if Choice.Id.equal (Choice.id item) (Choice.Id.of_string "section" |> ok)
               then 700
               else 400)
          ])
      ~on_navigate:Choice.Id.to_string
      ()
    |> ok
  in
  let reconciler = Reconciler.create window in
  let initial = commit reconciler (render ~passive:false) in
  let home = button initial "Home"
  and section = button initial "Section" in
  ignore (commit reconciler (render ~passive:true) : W.Op.t list);
  assert (Option.is_none (dispatch reconciler section));
  assert (Option.equal String.equal (dispatch reconciler home) (Some "root"));
  print_endline "passive intermediate has no action; unaffected route remains live";
  [%expect {| passive intermediate has no action; unaffected route remains live |}]
;;

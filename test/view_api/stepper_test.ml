open Core
open Gpuio
module W = Gpuio_protocol.Wire
module A = Gpuio_protocol.Accessibility_wire

let ok = Or_error.ok_exn
let id value = Choice.Id.of_string value |> ok

let choices entries =
  List.map entries ~f:(fun (name, disabled) ->
    Choice.create ~id:(id name) ~label:name ~disabled () |> ok)
  |> Choice.Collection.create
  |> ok
;;

let entries = [ "Draft", false; "Approval", true; "Review", false; "Publish", false ]

let create ?(current = Some "Draft") entries =
  Stepper.create ~steps:(choices entries) ~current:(Option.map current ~f:id) () |> ok
;;

let current t = Stepper.current t |> Option.map ~f:Choice.Id.to_string
let assert_current t name = assert (Option.equal String.equal (current t) name)

let%expect_test "latest model controls queued navigation and positional status" =
  let initial = create entries in
  let next = Stepper.apply_request initial Next in
  assert_current next (Some "Review");
  let last = Stepper.apply_request next Next in
  assert_current last (Some "Publish");
  assert (Stepper.equal last (Stepper.apply_request last Next));
  assert_current (Stepper.apply_request last Previous) (Some "Review");
  assert (Stepper.equal initial (Stepper.apply_request initial Previous));
  assert (Stepper.equal initial (Stepper.apply_request initial (Select (id "Approval"))));
  let disabled = Stepper.with_disabled next true in
  List.iter
    [ Stepper.Request.Next; Previous; Select (id "Publish") ]
    ~f:(fun request ->
      assert (Stepper.equal disabled (Stepper.apply_request disabled request)));
  let reordered =
    Stepper.with_steps next (choices [ "Review", false; "Draft", false ]) |> ok
  in
  assert_current reordered (Some "Review");
  assert (
    Option.equal
      Stepper.Status.equal
      (Stepper.status reordered (id "Draft"))
      (Some Upcoming));
  assert (
    Stepper.equal reordered (Stepper.apply_request reordered (Select (id "Publish"))));
  let removed = Stepper.with_steps reordered (choices [ "Draft", false ]) |> ok in
  assert_current removed None;
  assert_current (Stepper.apply_request removed Next) (Some "Draft");
  let unstarted = create ~current:None entries in
  assert_current (Stepper.apply_request unstarted Previous) (Some "Publish");
  assert_current (Stepper.select disabled (Some (id "Approval")) |> ok) (Some "Approval");
  print_s
    [%sexp
      (List.map entries ~f:(fun (name, _) -> name, Stepper.status next (id name))
       : (string * Stepper.Status.t option) list)];
  [%expect
    {|
    ((Draft (Completed)) (Approval (Completed)) (Review (Current))
     (Publish (Upcoming)))
    |}]
;;

let%expect_test "empty and maximum models; labels and dimensions are validated" =
  let empty = create ~current:None [] in
  assert_current (Stepper.apply_request empty Next) None;
  assert_current (Stepper.apply_request empty Previous) None;
  assert (Result.is_error (Stepper.select empty (Some (id "absent"))));
  List.iter
    [ "  "; String.make 1025 'x' ]
    ~f:(fun label ->
      let steps =
        Choice.Collection.create [ Choice.create ~id:(id "valid") ~label () |> ok ] |> ok
      in
      assert (Result.is_error (Stepper.create ~steps ~current:None ())));
  let all_disabled = create ~current:None [ "A", true; "B", true ] in
  assert_current (Stepper.apply_request all_disabled Next) None;
  let many n = choices (List.init n ~f:(fun i -> Int.to_string i, false)) in
  assert (Result.is_ok (Stepper.create ~steps:(many 64) ~current:None ()));
  assert (Result.is_error (Stepper.create ~steps:(many 65) ~current:None ()));
  List.iter [ Float.nan; Float.infinity; -1.; 4097. ] ~f:(fun bad ->
    assert (Result.is_error (Stepper.Appearance.create ~indicator_size:bad ()));
    assert (Result.is_error (Stepper.Appearance.create ~gap:bad ()));
    assert (Result.is_error (Stepper.Appearance.create ~connector_thickness:bad ())));
  assert (Result.is_error (Stepper.Appearance.create ~indicator_size:0. ()));
  assert (Result.is_error (Stepper.Appearance.create ~connector_thickness:0. ()));
  assert (Result.is_ok (Stepper.Appearance.create ~gap:0. ()));
  assert (Result.is_error (Stepper.view empty ~label:"" ~on_request:Fn.id ()));
  let bad_labels =
    Stepper.Labels.create ~description:(fun ~index:_ ~count:_ _ -> "\000")
  in
  assert (
    Result.is_error
      (Stepper.view
         (create entries)
         ~labels:bad_labels
         ~label:"Stages"
         ~on_request:Fn.id
         ()));
  print_endline
    "empty, disabled, 64-step boundary, finite geometry and localized text checked";
  [%expect
    {| empty, disabled, 64-step boundary, finite geometry and localized text checked |}]
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

let render ?axis ?indicator ?content ?labels model =
  Stepper.view
    model
    ?axis
    ?indicator
    ?content
    ?labels
    ~label:"Publishing stages"
    ~on_request:Fn.id
    ()
  |> ok
;;

let button operations label =
  List.find_map_exn operations ~f:(function
    | W.Op.Create (node, Button, text, Some handler) when String.equal text label ->
      Some (node, handler)
    | _ -> None)
;;

let dispatch reconciler (node, handler) =
  Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L))
;;

let%expect_test "queued events recheck policy; layout and labels retain native identity" =
  let model = create entries in
  let reconciler = Reconciler.create window in
  let initial = commit reconciler (render model) in
  let target = button initial "Review" in
  let request = dispatch reconciler target |> Option.value_exn in
  let disabled = Stepper.with_disabled model true in
  assert (Stepper.equal disabled (Stepper.apply_request disabled request));
  let options =
    Choice.Collection.create
      [ Choice.create ~id:(id "Review") ~label:"Inspect" () |> ok
      ; Choice.create ~id:(id "Draft") ~label:"Edit" () |> ok
      ]
    |> ok
  in
  let reordered = Stepper.with_steps model options |> ok in
  let update = commit reconciler (render ~axis:Vertical reordered) in
  assert (
    not
      (List.exists update ~f:(function
         | W.Op.Remove node -> Gpuio_protocol.Node_id.equal node (fst target)
         | _ -> false)));
  let request = dispatch reconciler target |> Option.value_exn in
  assert_current (Stepper.apply_request reordered request) (Some "Review");
  ignore (commit reconciler (render (Stepper.with_disabled reordered true)) : W.Op.t list);
  assert (Option.is_none (dispatch reconciler target));
  ignore (commit reconciler (render (create ~current:None [])) : W.Op.t list);
  assert (Option.is_none (dispatch reconciler target));
  print_endline
    "stable IDs survive reorder/rename/orientation; disable and removal retire activation";
  [%expect
    {| stable IDs survive reorder/rename/orientation; disable and removal retire activation |}]
;;

let%expect_test
    "rich triggers carry current-step and position semantics without extra handlers"
  =
  let model = create ~current:(Some "Review") entries in
  let view = render model in
  let metadata view =
    (View.Expert.describe view).accessibility
    |> Option.value_exn
    |> Accessibility.Expert.to_wire
  in
  assert (Option.equal A.Role.equal (metadata view).role (Some Navigation));
  List.iteri (View.Expert.describe view).children ~f:(fun index group ->
    let children = (View.Expert.describe group).children in
    assert (List.length children = if index = 3 then 1 else 2);
    let trigger = List.hd_exn children in
    let meta = metadata trigger in
    assert (
      Option.equal A.Current.equal meta.current (if index = 2 then Some Step else None));
    print_endline (Option.value_exn meta.description));
  let operations = commit (Reconciler.create window) view in
  assert (
    List.count operations ~f:(function
      | W.Op.Create (_, _, _, Some _) -> true
      | _ -> false)
    = 3);
  let interactive _ _ = View.button ~on_click:(fun () -> Stepper.Request.Next) "Nested" in
  assert (
    Result.is_error
      (Stepper.view model ~indicator:interactive ~label:"Stages" ~on_request:Fn.id ()));
  assert (
    Result.is_error
      (Stepper.view model ~content:interactive ~label:"Stages" ~on_request:Fn.id ()));
  [%expect
    {|
    Step 1 of 4 · Completed step
    Step 2 of 4 · Completed step
    Step 3 of 4 · Current step
    Step 4 of 4 · Upcoming step
    |}]
;;

let%expect_test "public workflow transaction fixture" =
  let view = render (create ~current:(Some "Review") entries) in
  let r = Reconciler.create window in
  let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
  let message = Reconciler.message update |> Option.value_exn in
  let bytes =
    Bin_prot.Utils.bin_dump W.Message.bin_writer_t message |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  let fixture =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "stepper-public-view.hex")
      |> String.strip)
  in
  assert (String.equal hex fixture);
  [%expect {| |}]
;;

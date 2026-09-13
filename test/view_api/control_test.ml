open Core
open Gpuio
open Gpuio_protocol

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let commit t view =
  let update = Reconciler.prepare t ~theme:Theme.default (Some view) |> Or_error.ok_exn in
  Reconciler.accept t update |> Or_error.ok_exn;
  match Reconciler.message update with
  | Some (Wire.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let binding ops =
  List.find_map_exn ops ~f:(function
    | Wire.Op.Create (node, _, _, Some handler) -> Some (node, handler)
    | _ -> None)
;;

let checkbox ?(disabled = false) state =
  View.checkbox
    ~key:(Key.of_string_exn "check")
    ~disabled
    ~state
    ~on_toggle:(fun () -> `Activate)
    "Stream responses"
;;

let%expect_test "rapid activations use the current model; values update without remount" =
  let t = Reconciler.create window in
  let node, handler = commit t (checkbox Unchecked) |> binding in
  let event = Wire.Event.Press (window, node, handler, 1L) in
  let model = ref Check_state.Unchecked in
  for _ = 1 to 2 do
    match Reconciler.dispatch t event with
    | Some `Activate -> model := Check_state.activate !model
    | None -> assert false
  done;
  print_s [%sexp (!model : Check_state.t)];
  assert (List.is_empty (commit t (checkbox !model)));
  let ops = commit t (checkbox Indeterminate) in
  print_s [%sexp (ops : Wire.Op.t list)];
  assert (Option.is_some (Reconciler.dispatch t event));
  [%expect
    {|
    Unchecked
    ((Set_control ((slot 0) (generation 1)) (Checkbox Indeterminate false)))
    |}]
;;

let%expect_test
    "disable invalidates queued activations and re-enable creates a fresh handler"
  =
  let t = Reconciler.create window in
  let node, handler = commit t (checkbox Checked) |> binding in
  let stale = Wire.Event.Press (window, node, handler, 1L) in
  let disabled = commit t (checkbox ~disabled:true Checked) in
  assert (Option.is_none (Reconciler.dispatch t stale));
  let enabled = commit t (checkbox Checked) in
  let next =
    List.find_map_exn enabled ~f:(function
      | Wire.Op.Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (not (Handler_id.equal handler next));
  assert (Option.is_none (Reconciler.dispatch t stale));
  assert (
    Option.is_some (Reconciler.dispatch t (Wire.Event.Press (window, node, next, 3L))));
  print_s [%sexp (disabled : Wire.Op.t list)];
  print_endline "fresh handler, retained node";
  [%expect
    {|
    ((Bind ((slot 0) (generation 1)) ())
     (Set_control ((slot 0) (generation 1)) (Checkbox Checked true)))
    fresh handler, retained node
    |}]
;;

let%expect_test
    "switch values and callbacks update independently; kind replacement expires identity"
  =
  let t = Reconciler.create window in
  let make checked action =
    View.switch
      ~key:(Key.of_string_exn "check")
      ~checked
      ~on_toggle:(fun () -> action)
      "Stream responses"
  in
  let node, handler = commit t (make false `First) |> binding in
  assert (List.is_empty (commit t (make false `Latest)));
  let latest = Reconciler.dispatch t (Wire.Event.Press (window, node, handler, 1L)) in
  (match latest with
   | Some `Latest -> ()
   | Some `First | None -> assert false);
  let ops = commit t (make true `Latest) in
  print_s [%sexp (ops : Wire.Op.t list)];
  let replacement =
    commit
      t
      (View.checkbox
         ~key:(Key.of_string_exn "check")
         ~state:Checked
         ~on_toggle:(fun () -> `Latest)
         "Stream responses")
  in
  assert (not (Node_id.equal node (fst (binding replacement))));
  assert (
    Option.is_none (Reconciler.dispatch t (Wire.Event.Press (window, node, handler, 1L))));
  [%expect {| ((Set_control ((slot 0) (generation 1)) (Switch true false))) |}]
;;

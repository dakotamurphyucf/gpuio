open Core
open Gpuio
module F = Settings_field
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let field ?help ?error () =
  Accessibility.Field.create ~label:"Preference" ?help ?error () |> ok
;;

let choice ?disabled id label =
  Choice.create ~id:(Choice.Id.of_string id |> ok) ~label ?disabled () |> ok
;;

let choices entries = F.Choices.create (module Int) entries |> ok

let rec descriptions view =
  let d = View.Expert.describe view in
  d :: List.concat_map d.children ~f:descriptions
;;

let commit reconciler view =
  let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept reconciler update |> ok;
  match Reconciler.message update with
  | Some (Apply tx) -> tx.operations
  | _ -> []
;;

let binding operations kind =
  List.find_map_exn operations ~f:(function
    | W.Op.Create (node, actual, _, Some handler) when W.Kind.equal actual kind ->
      Some (node, handler)
    | _ -> None)
;;

let%expect_test
    "Settings choices retain typed identity across relabel/reorder and reject ambiguity"
  =
  let options =
    choices
      [ 42, choice "answer" "Same label"; 7, choice ~disabled:true "locked" "Same label" ]
  in
  let view options ?(disabled = false) ?error selected =
    F.Choices.select
      options
      ~field:(field ?error ())
      ~disabled
      ~layout:Horizontal
      ~selected
      ~on_select:Fn.id
      ()
    |> ok
  in
  assert (
    Or_error.is_error
      (F.Choices.create (module Int) [ 42, choice "a" "a"; 42, choice "b" "b" ]));
  assert (
    Or_error.is_error
      (F.Choices.create (module Int) [ 42, choice "a" "a"; 7, choice "a" "b" ]));
  assert (
    Or_error.is_error
      (F.Choices.select
         options
         ~field:(field ())
         ~layout:Horizontal
         ~selected:(Some 99)
         ~on_select:Fn.id
         ()));
  let reconciler = Reconciler.create window in
  let first = commit reconciler (view options (Some 7)) in
  let node, handler = binding first Select in
  let activate id =
    Reconciler.dispatch reconciler (W.Event.Choice (window, node, handler, 1L, id))
  in
  assert (Option.equal Int.equal (activate "answer") (Some 42));
  assert (Option.is_none (activate "locked"));
  let replacement =
    choices [ 7, choice "locked" "Now enabled"; 42, choice "answer" "Renamed" ]
  in
  let changes =
    commit reconciler (view replacement ~error:"Choose carefully" (Some 42))
  in
  assert (
    List.for_all changes ~f:(function
      | W.Op.Remove removed -> not (Gpuio_protocol.Node_id.equal node removed)
      | Create (_, Select, _, _) -> false
      | _ -> true));
  (* Dispatch obtains the latest accepted typed mapping and option policy. *)
  assert (Option.equal Int.equal (activate "locked") (Some 7));
  ignore (commit reconciler (view replacement ~disabled:true (Some 42)) : W.Op.t list);
  assert (Option.is_none (activate "answer"));
  Reconciler.close reconciler;
  print_endline
    "typed values survive labels/order; duplicate IDs/values and foreign selection \
     rejected; native disabled policy applies";
  [%expect
    {| typed values survive labels/order; duplicate IDs/values and foreign selection rejected; native disabled policy applies |}]
;;

let%expect_test "Settings Boolean controls dispatch toggle intents against current state" =
  List.iter [ F.switch; F.checkbox ] ~f:(fun make ->
    let reconciler = Reconciler.create window in
    let value = ref false in
    let view disabled =
      make
        (field ~help:"A preference" ())
        ~layout:Vertical
        ~disabled
        ~checked:!value
        ~on_toggle:(fun () -> ())
        ()
      |> ok
    in
    let operations = commit reconciler (view false) in
    let node, handler =
      List.find_map_exn operations ~f:(function
        | W.Op.Create (node, (Switch | Checkbox), _, Some handler) -> Some (node, handler)
        | _ -> None)
    in
    let event = W.Event.Press (window, node, handler, 1L) in
    List.iter [ (); () ] ~f:(fun () ->
      Option.iter (Reconciler.dispatch reconciler event) ~f:(fun () ->
        value := not !value));
    assert (not !value);
    assert (List.is_empty (commit reconciler (view false)));
    ignore (commit reconciler (view true) : W.Op.t list);
    assert (Option.is_none (Reconciler.dispatch reconciler event));
    Reconciler.close reconciler);
  print_endline
    "two queued toggle intents cancel; disabled state fences queued native activation";
  [%expect
    {| two queued toggle intents cancel; disabled state fences queued native activation |}]
;;

let%expect_test "Settings native controls retain placement when help and errors change" =
  let config = Text_input.Config.create ~mode:Single_line ~label:"Initial" () |> ok in
  let native =
    View.text_input
      ~controller:(Key.of_string_exn "name")
      ~initial_text:"draft"
      ~config
      ~on_event:(fun _ -> ())
      ()
    |> ok
  in
  let view ?help ?error layout =
    F.control (field ?help ?error ()) ~layout ~control:native () |> ok
  in
  let reconciler = Reconciler.create window in
  let node, _ = binding (commit reconciler (view Horizontal)) Input in
  let changes =
    commit reconciler (view ~help:"Helpful" ~error:"Invalid value" Vertical)
  in
  assert (
    List.for_all changes ~f:(function
      | W.Op.Remove removed -> not (Gpuio_protocol.Node_id.equal node removed)
      | Create (_, Input, _, _) -> false
      | Set_text (changed, _) -> not (Gpuio_protocol.Node_id.equal node changed)
      | _ -> true));
  let native =
    List.find_exn
      (descriptions (view ~help:"Helpful" ~error:"Invalid value" Vertical))
      ~f:(fun d -> Option.is_some d.editor)
  in
  let semantic =
    native.accessibility |> Option.value_exn |> Accessibility.Expert.to_wire
  in
  let field = semantic.field |> Option.value_exn in
  assert (String.equal field.label "Preference");
  assert (Option.equal String.equal field.help (Some "Helpful"));
  assert (Option.equal String.equal field.error (Some "Invalid value"));
  assert (
    Or_error.is_error
      (F.control
         (Accessibility.Field.create ~label:"Bad root" () |> ok)
         ~layout:Vertical
         ~control:(View.text "not an input")
         ()));
  Reconciler.close reconciler;
  print_endline
    "native name/help/error attached; editor survives layout and annotations; \
     unsupported roots rejected";
  [%expect
    {| native name/help/error attached; editor survives layout and annotations; unsupported roots rejected |}]
;;

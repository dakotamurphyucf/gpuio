open Core
open Gpuio
module W = Gpuio_protocol.Wire

let%expect_test "button icons are decorative and optional slots retain identity" =
  let owner = Asset.Expert.Owner.create () in
  let resource =
    Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let asset = Asset.Expert.handle ~owner ~id:resource ~format:Svg in
  let icon = Icon.Decoration.create ~asset () |> Or_error.ok_exn in
  let raster = Asset.Expert.handle ~owner ~id:resource ~format:Png in
  assert (Result.is_error (Icon.Decoration.create ~asset:raster ()));
  let button ?leading_icon ?trailing_icon () =
    View.button ?leading_icon ?trailing_icon ~on_click:(fun () -> "send") "Send"
  in
  let decorated = button ~leading_icon:icon ~trailing_icon:icon () in
  let description = View.Expert.describe decorated in
  assert (String.equal description.text "Send");
  assert (Option.is_some description.on_click);
  List.iter description.children ~f:(fun slot ->
    let slot = View.Expert.describe slot in
    assert (Option.is_none slot.on_click);
    let child = View.Expert.describe (List.hd_exn slot.children) in
    assert (View.Expert.Kind.equal child.kind Icon);
    assert (Option.is_none child.on_click);
    let image = Option.value_exn child.image in
    assert (Option.is_none image.on_change);
    assert (Option.is_none (Image.Expert.label (Image.Config.description image.config))));
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let reconciler = Reconciler.create ~asset_owner:owner window in
  let prepare view =
    Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
  in
  let initial = prepare decorated in
  let operations = function
    | Some (W.Message.Apply { operations; _ }) -> operations
    | _ -> assert false
  in
  let initial_ops = operations (Reconciler.message initial) in
  let root, handler =
    List.find_map_exn initial_ops ~f:(function
      | W.Op.Create (id, Button, _, Some handler) -> Some (id, handler)
      | _ -> None)
  in
  let icons =
    List.filter_map initial_ops ~f:(function
      | W.Op.Create (id, Icon, _, None) -> Some id
      | _ -> None)
  in
  assert (List.length icons = 2);
  Reconciler.accept reconciler initial |> Or_error.ok_exn;
  let trailing = prepare (button ~trailing_icon:icon ()) in
  let operations = operations (Reconciler.message trailing) in
  assert (
    not
      (List.exists operations ~f:(function
         | W.Op.Create _ | Bind _ -> true
         | _ -> false)));
  assert (
    List.exists operations ~f:(function
      | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id (List.hd_exn icons)
      | _ -> false));
  assert (
    not
      (List.exists operations ~f:(function
         | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id (List.last_exn icons)
         | _ -> false)));
  Reconciler.accept reconciler trailing |> Or_error.ok_exn;
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch reconciler (W.Event.Press (window, root, handler, 1L)))
      (Some "send"));
  let icon_only = View.icon_button ~label:"Send" ~on_click:(fun () -> ()) icon in
  assert (String.is_empty (View.Expert.describe icon_only).text);
  assert (
    Result.is_error
      (Or_error.try_with (fun () ->
         View.icon_button ~label:"" ~on_click:(fun () -> ()) icon)));
  let disabled =
    View.icon_button ~disabled:true ~label:"Send" ~on_click:(fun () -> ()) icon
  in
  assert (Option.is_none (View.Expert.describe disabled).on_click);
  print_endline
    "decorative SVG slots; stable trailing icon and button action; labelled icon-only \
     button; disabled action removed";
  [%expect
    {| decorative SVG slots; stable trailing icon and button action; labelled icon-only button; disabled action removed |}]
;;

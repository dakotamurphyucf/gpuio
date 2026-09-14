open Core
open Gpuio
module W = Gpuio_protocol.Wire

let%expect_test "icons require SVG; replacing an image with an icon remounts it" =
  let owner = Asset.Expert.Owner.create () in
  let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let asset = Asset.Expert.handle ~owner ~id ~format:Svg in
  let description = Image.Description.label "Search" |> Or_error.ok_exn in
  let raster = Asset.Expert.handle ~owner ~id ~format:Png in
  print_s
    [%sexp (Icon.Config.create ~asset:raster ~description () : Icon.Config.t Or_error.t)];
  let config = Icon.Config.create ~asset ~description () |> Or_error.ok_exn in
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let reconciler = Reconciler.create ~asset_owner:owner window in
  let prepare view =
    Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
  in
  let image = prepare (View.image (Image.Config.create ~asset ~description ())) in
  let image_id =
    match Reconciler.message image with
    | Some (Apply { operations; _ }) ->
      List.find_map_exn operations ~f:(function
        | W.Op.Create (id, Image, _, _) -> Some id
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler image |> Or_error.ok_exn;
  let icon = prepare (View.icon ~on_change:Fn.id config) in
  (match Reconciler.message icon with
   | Some (Apply { operations; _ }) ->
     assert (
       List.exists operations ~f:(function
         | W.Op.Remove id -> Gpuio_protocol.Node_id.equal image_id id
         | _ -> false));
     assert (
       List.exists operations ~f:(function
         | W.Op.Create (id, Icon, _, Some _) ->
           not (Gpuio_protocol.Node_id.equal image_id id)
         | _ -> false));
     assert (
       List.exists operations ~f:(function
         | W.Op.Set_image
             (_, { source = Reference found; fit = Contain; label = Some "Search" }) ->
           Gpuio_protocol.Resource_id.equal id found
         | _ -> false))
   | _ -> assert false);
  Reconciler.accept reconciler icon |> Or_error.ok_exn;
  let refreshed = prepare (View.icon ~on_change:Fn.id config) in
  assert (Option.is_none (Reconciler.message refreshed));
  print_endline "image-to-icon remount; default fit/label retained; stable icon refresh";
  [%expect
    {| 
    (Error "icons require an SVG asset")
    image-to-icon remount; default fit/label retained; stable icon refresh
    |}]
;;

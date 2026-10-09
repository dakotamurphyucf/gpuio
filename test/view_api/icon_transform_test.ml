open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "bounded transform construction and independent operation bytes" =
  let transform =
    Icon.Transform.create
      ~scale_x:1.5
      ~scale_y:(-0.5)
      ~rotation_degrees:90.
      ~translate_x:3.
      ~translate_y:(-4.)
      ()
    |> ok
  in
  assert (Icon.Transform.equal Icon.Transform.identity (Icon.Transform.create () |> ok));
  assert (
    Icon.Transform.equal
      (Icon.Transform.rotate_degrees 90. |> ok)
      (Icon.Transform.create ~rotation_degrees:90. () |> ok));
  List.iter
    [ (fun v -> Icon.Transform.create ~scale_x:v ()), 64.
    ; (fun v -> Icon.Transform.create ~scale_y:v ()), 64.
    ; (fun v -> Icon.Transform.create ~rotation_degrees:v ()), 360.
    ; (fun v -> Icon.Transform.create ~translate_x:v ()), 16384.
    ; (fun v -> Icon.Transform.create ~translate_y:v ()), 16384.
    ]
    ~f:(fun (create, bound) ->
      List.iter [ 0.; bound; -.bound ] ~f:(fun v -> assert (Result.is_ok (create v)));
      List.iter
        [ bound +. 1.; -.bound -. 1.; Float.nan; Float.infinity; Float.neg_infinity ]
        ~f:(fun v -> assert (Result.is_error (create v))));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:1L |> ok in
  Eio_main.run (fun env ->
    List.iter
      [ Some transform, "icon-transform.hex"; None, "icon-transform-clear.hex" ]
      ~f:(fun (value, file) ->
        let message =
          W.Message.Apply
            { window
            ; base = 0L
            ; revision = 1L
            ; operations =
                [ W.Op.Set_icon_transform
                    (node, Option.map value ~f:Icon.Expert.transform_to_wire)
                ]
            }
        in
        let expected =
          Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip
        in
        let bytes = W.Message.encode message |> ok in
        let hex =
          String.to_list bytes
          |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
          |> String.concat
        in
        assert (String.equal hex expected)));
  print_endline
    "bounded SRT, identity, rotation convenience and independent tag-128/reset bytes";
  [%expect
    {| bounded SRT, identity, rotation convenience and independent tag-128/reset bytes |}]
;;

let%expect_test
    "transform-only updates preserve icon and source identity, with explicit reset"
  =
  let owner = Asset.Expert.Owner.create () in
  let resource = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let asset = Asset.Expert.handle ~owner ~id:resource ~format:Svg in
  let config =
    Icon.Config.create ~asset ~description:Image.Description.decorative () |> ok
  in
  let rotated = Icon.Transform.rotate_degrees 90. |> ok in
  let transformed = Icon.Config.with_transform config (Some rotated) in
  assert (Icon.Config.equal config (Icon.Config.with_transform transformed None));
  assert (Image.Config.equal (Icon.Expert.image config) (Icon.Expert.image transformed));
  let decoration = Icon.Decoration.create ~asset ~transform:rotated () |> ok in
  let decorated_config, _ = Icon.Expert.decoration decoration in
  assert (Icon.Config.equal transformed decorated_config);
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~asset_owner:owner window in
  let prepare config =
    Reconciler.prepare reconciler ~theme:Theme.default (Some (View.icon config)) |> ok
  in
  let initial = prepare config in
  Reconciler.accept reconciler initial |> ok;
  List.iter
    [ transformed, Some (Icon.Expert.transform_to_wire rotated); config, None ]
    ~f:(fun (config, expected) ->
      let prepared = prepare config in
      (match Reconciler.message prepared with
       | Some (Apply { operations = [ Set_icon_transform (_, actual) ]; _ }) ->
         assert (Option.equal Gpuio_protocol.Icon_transform_wire.equal actual expected)
       | _ -> assert false);
      Reconciler.accept reconciler prepared |> ok;
      assert (Option.is_none (Reconciler.message (prepare config))));
  print_endline
    "source/config preserved; decoration forwards transform; only transform/reset sent; \
     refresh silent";
  [%expect
    {| source/config preserved; decoration forwards transform; only transform/reset sent; refresh silent |}]
;;

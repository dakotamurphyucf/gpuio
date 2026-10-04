open Core
open Gpuio
module W = Gpuio_protocol.Wire
module G = Gpuio_protocol.Placement_geometry_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "public help and menu helpers submit point geometry; dialogs ignore it" =
  let placement = Placement.at_point ~x:400. ~y:300. () |> ok in
  let anchor = View.button ~on_click:(fun () -> ()) "Open" in
  let content = View.text "Details" in
  let cases =
    [ ( "tooltip"
      , true
      , View.tooltip
          ~config:(Tooltip.Config.create ~label:"Help" ~placement () |> ok)
          ~anchor
          ~content
          () )
    ; ( "hover card"
      , true
      , View.hover_card
          ~config:(Hover_card.Config.create ~label:"Preview" ~placement () |> ok)
          ~anchor
          ~content
          () )
    ; ( "menu"
      , true
      , View.menu_button ~placement ~menu:(Menu.create ~label:"Actions" [] |> ok) () )
    ; ( "dialog"
      , false
      , View.dialog
          ~config:(Overlay.Config.create ~label:"Modal" ~placement () |> ok)
          ~on_dismiss:ignore
          (Some content) )
    ]
  in
  List.iter cases ~f:(fun (name, expected, view) ->
    let tx =
      Reconciler.prepare (Reconciler.create window) ~theme:Theme.default (Some view) |> ok
    in
    let operations =
      match Reconciler.message tx with
      | Some (W.Message.Apply tx) -> tx.operations
      | None | Some _ -> assert false
    in
    let geometry =
      List.filter_map operations ~f:(function
        | W.Op.Set_placement_geometry (_, value) -> Some value
        | _ -> None)
    in
    assert (
      List.equal
        (Option.equal G.equal)
        geometry
        (if expected then [ Placement.Expert.geometry placement ] else []));
    print_s [%sexp (name : string), (expected : bool)]);
  [%expect
    {|
    (tooltip true)
    ("hover card" true)
    (menu true)
    (dialog false)
    |}]
;;

let%expect_test "point placement validates coordinates and margins" =
  List.iter
    [ Placement.create ~viewport_margin:(-1.) ()
    ; Placement.create ~viewport_margin:Float.nan ()
    ; Placement.create ~viewport_margin:16385. ()
    ; Placement.at_point ~x:Float.infinity ~y:0. ()
    ; Placement.at_point ~x:0. ~y:(-1_000_001.) ()
    ]
    ~f:(fun result -> assert (Or_error.is_error result));
  assert (Option.is_none (Placement.Expert.geometry Placement.default));
  List.iter
    [ Placement.Corner.Top_left; Top_right; Bottom_left; Bottom_right ]
    ~f:(fun corner ->
      let placement =
        Placement.at_point ~corner ~x:(-1_000_000.) ~y:1_000_000. ~viewport_margin:0. ()
        |> ok
      in
      assert (G.valid (Placement.Expert.geometry placement |> Option.value_exn)));
  print_endline
    "finite bounded geometry; all corners accepted; legacy default omits metadata";
  [%expect
    {| finite bounded geometry; all corners accepted; legacy default omits metadata |}]
;;

let%expect_test "point and margin updates retain popup ownership and reset to defaults" =
  let r = Reconciler.create window in
  let view placement =
    View.popover
      ~config:(Overlay.Config.create ~label:"Point preview" ~placement () |> ok)
      ~on_dismiss:ignore
      ~anchor:(View.button ~on_click:(fun () -> ()) "Open")
      (Some (View.button ~on_click:(fun () -> ()) "Keep"))
  in
  let update placement =
    let tx = Reconciler.prepare r ~theme:Theme.default (Some (view placement)) |> ok in
    Reconciler.accept r tx |> ok;
    match Reconciler.message tx with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = update (Placement.at_point ~x:100. ~y:120. () |> ok) in
  let node =
    List.find_map_exn first ~f:(function
      | W.Op.Set_placement_geometry (id, Some _) -> Some id
      | _ -> None)
  in
  List.iter
    [ Placement.at_point ~corner:Bottom_right ~x:400. ~y:300. ~viewport_margin:24. ()
      |> ok
    ; Placement.create ~viewport_margin:32. () |> ok
    ; Placement.default
    ]
    ~f:(fun placement ->
      let ops = update placement in
      assert (
        List.exists ops ~f:(function
          | W.Op.Set_placement_geometry (id, geometry) ->
            Gpuio_protocol.Node_id.equal id node
            && Option.equal G.equal geometry (Placement.Expert.geometry placement)
          | _ -> false));
      assert (
        not
          (List.exists ops ~f:(function
             | W.Op.Create _ | Remove _ | Set_overlay _ | Set_focus_scope _ -> true
             | _ -> false))));
  print_endline
    "point/corner/margin/reset change geometry only; panel and children retained";
  [%expect
    {| point/corner/margin/reset change geometry only; panel and children retained |}]
;;

let%expect_test "all point corners, margin-only and reset match independent bytes" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let operations =
    List.map [ G.Corner.Top_left; Top_right; Bottom_left; Bottom_right ] ~f:(fun corner ->
      W.Op.Set_placement_geometry
        ( node
        , Some { viewport_margin = 24.5; point = Some { corner; x = 125.5; y = -12.25 } }
        ))
    @ [ W.Op.Set_placement_geometry (node, Some { viewport_margin = 0.; point = None })
      ; Set_placement_geometry (node, None)
      ]
  in
  let bytes =
    W.Message.encode (Apply { window; base = 0L; revision = 1L; operations }) |> ok
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "placement-geometry.hex")
         |> String.strip)));
  print_endline "Op93 geometry/reset match independent bytes";
  [%expect {| Op93 geometry/reset match independent bytes |}]
;;

open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn
let id = Tab_menu_test.id
let owner = Asset.Expert.Owner.create ()

let decoration ?(color = 0x22aa66) slot =
  let asset =
    Asset.Expert.handle
      ~owner
      ~id:(P.Resource_id.create ~slot ~generation:1L |> ok)
      ~format:Svg
  in
  Icon.Decoration.create
    ~asset
    ~style:(Style.create_exn [ Foreground (Color.rgb_exn color) ])
    ()
  |> ok
;;

let%expect_test "menu icons are bounded passive decorations tied to current choice IDs" =
  let icon = decoration 0L in
  assert (Result.is_error (Tab_bar.Menu.create ~icons:[ id 0, icon; id 0, icon ] ()));
  let asset =
    Asset.Expert.handle
      ~owner
      ~id:(P.Resource_id.create ~slot:0L ~generation:1L |> ok)
      ~format:Svg
  in
  let active =
    Icon.Decoration.create ~asset ~style:(Style.create_exn [ User_select true ]) () |> ok
  in
  assert (Result.is_error (Tab_bar.Menu.create ~icons:[ id 0, active ] ()));
  let config =
    Choice.Config.create
      ~label:"Only one"
      ~options:
        (Choice.Collection.create [ Choice.create ~id:(id 0) ~label:"Zero" () |> ok ]
         |> ok)
      ~selected:None
      ()
    |> ok
  in
  let tabs = View.tab_bar ~config ~on_select:Choice.Id.to_string () in
  let menu = Tab_bar.Menu.create ~icons:[ id 39, icon ] () |> ok in
  assert (Result.is_error (View.tab_bar_frame ~menu tabs));
  let too_many = List.init (Choice.Collection.max_choices + 1) ~f:(fun n -> id n, icon) in
  assert (Result.is_error (Tab_bar.Menu.create ~icons:too_many ()));
  print_endline "duplicate/unknown IDs, active styles and oversized maps rejected";
  [%expect {| duplicate/unknown IDs, active styles and oversized maps rejected |}]
;;

let%expect_test "public menu icon lease placements and reset transactions" =
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create ~asset_owner:owner window in
  let view = Tab_menu_test.framed in
  let icons = [ id 0, decoration 0L; id 39, decoration 0L ] in
  let changed = [ id 0, decoration ~color:0xaa2266 1L; id 39, decoration 0L ] in
  let single = List.take changed 1 in
  let actual =
    List.map
      [ Some (view ~menu_icons:icons ())
      ; Some (view ~menu_icons:icons ~changed:true ())
      ; Some (view ~menu_icons:changed ~changed:true ())
      ; Some (view ~menu_icons:single ~changed:true ())
      ; Some (view ~changed:true ())
      ; Some (view ~menu_icons:single ~changed:true ())
      ; Some (view ~menu:false ~changed:true ())
      ; None
      ]
      ~f:(fun view ->
        let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
        Reconciler.accept r update |> ok;
        Reconciler.message update
        |> Option.value_exn
        |> W.Message.encode
        |> ok
        |> String.to_list
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat)
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "tab-menu-icons-transactions.hex")
      |> String.split_lines)
  in
  assert (List.equal String.equal expected actual);
  print_endline
    "public icon insertion, reorder, asset/style replacement, partial/full reset, re-add \
     and disposal";
  [%expect
    {| public icon insertion, reorder, asset/style replacement, partial/full reset, re-add and disposal |}]
;;

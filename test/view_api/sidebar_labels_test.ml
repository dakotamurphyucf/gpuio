open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let id text = Sidebar.Id.of_string text |> ok
let key = Key.of_string_exn
let style = Style.create_exn
let px = Length.px_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let group item = Sidebar.Group.create ~id:(id "group") [ item ] |> ok
let item label = Sidebar.Item.create ~id:(id "destination") ~label () |> ok

let model ?(label = "Destination") () =
  Sidebar.create ~groups:[ group (item label) ] ~selected:None () |> ok
;;

let%expect_test
    "sidebar names preserve the complete Unicode domain and styles are passive"
  =
  List.iter [ ""; " "; "\t\r\n" ] ~f:(fun label ->
    assert (Result.is_error (Sidebar.Item.create ~id:(id "invalid") ~label ())));
  let label = String.concat (List.init 2048 ~f:(fun _ -> "é")) in
  let t = model ~label () in
  let view =
    Sidebar.view
      t
      ~hidden:Retain
      ~on_request:Fn.id
      ~decorate:(fun _ ->
        Sidebar.Decoration.create ~label_style:(style [ Font_weight 700 ]) ())
      ()
    |> ok
  in
  let r = Reconciler.create window in
  let u = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
  let ops =
    match Reconciler.message u with
    | Some (Apply tx) -> tx.operations
    | _ -> assert false
  in
  assert (
    List.exists ops ~f:(function
      | W.Op.Set_link (_, config) -> String.equal config.label label
      | _ -> false));
  List.iter
    [ Style.Property.User_select true; Overflow_y Scroll; Inert true; Disabled true ]
    ~f:(fun property ->
      assert (
        Result.is_error
          (Sidebar.view
             (model ())
             ~hidden:Retain
             ~on_request:Fn.id
             ~decorate:(fun _ ->
               Sidebar.Decoration.create ~label_style:(style [ property ]) ())
             ())));
  print_endline
    "4096-byte Unicode names preserved; blank names and interactive label styles rejected";
  [%expect
    {| 4096-byte Unicode names preserved; blank names and interactive label styles rejected |}]
;;

let fixture_view index =
  let t = model () in
  let t = if index = 3 then Sidebar.with_disabled t true else t in
  let t = if index = 4 then Sidebar.with_collapsed t true else t in
  let label_style =
    style
      [ Background
          (Background.solid (Color.rgb_exn (if index = 1 then 0x22cc44 else 0xff2255)))
      ; Font_size (if index = 1 then 24. else 16.)
      ; Padding (px 3.)
      ]
  in
  let label_style =
    Style.with_state_exn
      label_style
      Disabled
      [ Background (Background.solid (Color.rgb_exn 0x5522ff)) ]
  in
  let label_style = if index = 2 then Style.empty else label_style in
  Sidebar.view
    t
    ~key:(key "sidebar")
    ~hidden:Retain
    ~on_request:Fn.id
    ~appearance:
      (Sidebar.Appearance.create
         ~motion:Sidebar.Motion.immediate
         ~width:240.
         ~compact_width:56.
         ()
       |> ok)
    ~decorate:(fun _ ->
      Sidebar.Decoration.create
        ~label_style
        ~style:(style [ Background (Background.solid (Color.rgb_exn 0x112244)) ])
        ~suffix:(View.text "Suffix")
        ())
    ()
  |> ok
;;

let%expect_test
    "sidebar public style transitions retain the destination and passive slots"
  =
  let r = Reconciler.create window in
  let targets = ref [] in
  for index = 0 to 5 do
    let view = if index = 5 then None else Some (fixture_view index) in
    let u = Reconciler.prepare r ~theme:Theme.default view |> ok in
    let message = Reconciler.message u |> Option.value_exn in
    Reconciler.accept r u |> ok;
    let operations =
      match message with
      | Apply tx -> tx.operations
      | _ -> assert false
    in
    if index = 0
    then
      targets
      := List.filter_map operations ~f:(function
           | W.Op.Create (node, Link, _, Some handler) -> Some (node, handler)
           | _ -> None)
    else if index < 5
    then
      assert (
        not
          (List.exists operations ~f:(function
             | W.Op.Create _ | Remove _ -> true
             | _ -> false)));
    let node, previous_handler = List.hd_exn !targets in
    let handler =
      List.find_map operations ~f:(function
        | W.Op.Bind (owner, Some handler) when Gpuio_protocol.Node_id.equal owner node ->
          Some handler
        | _ -> None)
      |> Option.value ~default:previous_handler
    in
    if index = 4
    then
      assert (
        Option.is_none
          (Reconciler.dispatch
             r
             (W.Event.Press (window, node, previous_handler, Reconciler.revision r))));
    targets := [ node, handler ];
    let action =
      Reconciler.dispatch r (W.Event.Press (window, node, handler, Reconciler.revision r))
    in
    if index = 3 || index = 5
    then assert (Option.is_none action)
    else
      assert (Option.equal Sidebar.Request.equal action (Some (Select (id "destination"))));
    let hex =
      W.Message.encode message
      |> ok
      |> String.to_list
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat
    in
    Eio_main.run (fun env ->
      let expected =
        Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / sprintf "sidebar-labels-%d.hex" index)
        |> String.strip
      in
      assert (String.equal hex expected))
  done;
  [%expect {| |}]
;;

let%expect_test "sidebar label style is independent of icon styling" =
  let owner = Asset.Expert.Owner.create () in
  let resource = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let asset = Asset.Expert.handle ~owner ~id:resource ~format:Svg in
  let icon = Icon.Decoration.create ~asset () |> ok in
  let label_style = style [ Foreground (Color.rgb_exn 0xff2255); Font_weight 700 ] in
  let build compact =
    Sidebar.view
      (Sidebar.with_collapsed (model ()) compact)
      ~hidden:Retain
      ~on_request:Fn.id
      ~decorate:(fun _ -> Sidebar.Decoration.create ~icon ~label_style ())
      ()
    |> ok
  in
  let rec find_link view =
    let d = View.Expert.describe view in
    if View.Expert.Kind.equal d.kind Link
    then Some d
    else List.find_map d.children ~f:find_link
  in
  List.iter [ false; true ] ~f:(fun compact ->
    let link = find_link (build compact) |> Option.value_exn in
    assert (Option.is_some link.on_click);
    match link.children with
    | [ icon_slot; label ] ->
      let icon = View.Expert.describe icon_slot in
      let icon = View.Expert.describe (List.hd_exn icon.children) in
      let label = View.Expert.describe label in
      assert (View.Expert.Kind.equal icon.kind Icon);
      assert (not (Style.equal icon.style label_style));
      assert (Option.is_none icon.on_click);
      assert (Option.is_none label.on_click);
      if compact
      then assert (String.is_empty label.text)
      else (
        assert (String.equal label.text "Destination");
        assert (Style.equal label.style label_style))
    | _ -> assert false);
  let r = Reconciler.create ~asset_owner:owner window in
  let initial = Reconciler.prepare r ~theme:Theme.default (Some (build false)) |> ok in
  Reconciler.accept r initial |> ok;
  let compact = Reconciler.prepare r ~theme:Theme.default (Some (build true)) |> ok in
  (match Reconciler.message compact with
   | Some (Apply tx) ->
     assert (
       not
         (List.exists tx.operations ~f:(function
            | W.Op.Create _ | Remove _ | Bind _ -> true
            | _ -> false)))
   | _ -> assert false);
  [%expect {| |}]
;;

open Core
open Gpuio
module P = Presentation
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let commit t ?(theme = Theme.default) view =
  let update = Reconciler.prepare t ~theme (Some view) |> ok in
  Reconciler.accept t update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let rec descriptions view =
  let d = View.Expert.describe view in
  d :: List.concat_map d.children ~f:descriptions
;;

let gallery p text =
  let content = View.text text in
  View.column
    [ P.label text
    ; P.badge p ~tone:Success ~variant:Solid text
    ; P.tag p ~tone:Accent ~leading:content ~trailing:content text
    ; P.marker p ~tone:Warning text
    ; P.separator p ()
    ; P.separator p ~axis:Vertical ()
    ; P.group_box p ~header:content ~footer:content [ content ]
    ; P.settings_group p ~title:text ~description:text [ content ]
    ; P.description_list
        p
        [ P.Description.create ~key:(Key.of_int 0) ~term:text ~definition:content ]
    ; P.empty_state p ~title:text ~description:text ~actions:content ()
    ; P.alert p ~title:text ~icon:content [ content ]
    ; P.banner p ~live:Off ~title:text [ content ]
    ; P.shortcut_label p [ "⌘"; "Shift"; "K" ]
    ; P.status_bar p ~leading:content ~trailing:content ()
    ; P.attachment p ~preview:content ~name:text ~detail:text ~actions:content ()
    ; P.message p ~avatar:content ~author:text ~detail:text ~footer:content content
    ; P.bubble p content
    ; P.tool_result p ~title:text ~status:content ~actions:content content
    ]
;;

let%expect_test "presentation stays stateless and works without additional theme tokens" =
  let theme = Theme.create [] |> ok in
  List.iter [ P.Appearance.light; P.Appearance.dark ] ~f:(fun p ->
    List.iter
      [ ""; "名前 · العربية · 👩🏽‍💻"; String.make 10000 'x' ]
      ~f:(fun text ->
        let view = gallery p text in
        List.iter (descriptions view) ~f:(fun d ->
          assert (
            List.mem
              [ View.Expert.Kind.Text; Container ]
              d.kind
              ~equal:View.Expert.Kind.equal));
        let reconciler = Reconciler.create window in
        ignore (commit reconciler ~theme view : W.Op.t list);
        assert (List.is_empty (commit reconciler ~theme view));
        let update = Reconciler.prepare reconciler ~theme None |> ok in
        Reconciler.accept reconciler update |> ok));
  print_endline
    "light/dark; empty, localized and long text; no controllers; unchanged views are idle";
  [%expect
    {| light/dark; empty, localized and long text; no controllers; unchanged views are idle |}]
;;

let%expect_test "optional card slots and appearance updates preserve body controls" =
  let t = Reconciler.create window in
  let control =
    View.checkbox ~state:Unchecked ~on_toggle:(fun () -> `Toggle) "Live updates"
  in
  let card p extra =
    P.message p ~author:"Assistant" ?avatar:extra ?footer:extra control
  in
  let initial = commit t (card P.Appearance.light None) in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Checkbox, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  List.iter
    [ Some (View.text "Ready"); None ]
    ~f:(fun extra ->
      let operations = commit t (card P.Appearance.dark extra) in
      List.iter operations ~f:(function
        | W.Op.Remove removed -> assert (not (Gpuio_protocol.Node_id.equal node removed))
        | Create (_, Checkbox, _, _) | Set_control _ -> failwith "body control was reset"
        | _ -> ());
      assert (
        Option.is_some (Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L)))));
  print_endline "body identity and handler survive header/footer and theme changes";
  [%expect {| body identity and handler survive header/footer and theme changes |}]
;;

let%expect_test "group panel changes preserve controls and independently refine slots" =
  let t = Reconciler.create window in
  let group p ?variant ?header ?footer revision =
    let refine = Style.create_exn [ Padding_left (Length.px_exn 23.) ] in
    P.group_box
      p
      ?variant
      ~header_style:refine
      ~body_style:refine
      ~footer_style:refine
      ?header
      ?footer
      [ View.checkbox ~state:Checked ~on_toggle:(fun () -> revision) "Persistent choice" ]
  in
  let first = group P.Appearance.light "original" in
  let initial = commit t first in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Checkbox, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let body_event = W.Event.Press (window, node, handler, 1L) in
  assert (List.is_empty (commit t (group P.Appearance.light ~variant:Card "original")));
  let stale_actions = ref [] in
  List.iter [ P.Appearance.light; P.Appearance.dark ] ~f:(fun p ->
    List.iter [ P.Group_variant.Card; Plain; Filled; Outline ] ~f:(fun variant ->
      List.iter
        [ true, true; false, true; true, false; false, false ]
        ~f:(fun (h, f) ->
          let control name = View.button name ~on_click:(fun () -> name) in
          let view =
            group
              p
              ~variant
              ?header:(Option.some_if h (control "header"))
              ?footer:(Option.some_if f (control "footer"))
              "latest"
          in
          let operations = commit t view in
          List.iter operations ~f:(function
            | W.Op.Remove removed ->
              assert (not (Gpuio_protocol.Node_id.equal node removed))
            | Create (_, Checkbox, _, _) -> failwith "group control replaced"
            | Set_control (changed, _) ->
              assert (not (Gpuio_protocol.Node_id.equal node changed))
            | Create (node, Button, _, Some handler) ->
              stale_actions := W.Event.Press (window, node, handler, 1L) :: !stale_actions
            | _ -> ());
          assert (
            Option.equal String.equal (Reconciler.dispatch t body_event) (Some "latest"));
          List.iter !stale_actions ~f:(fun event ->
            Option.iter (Reconciler.dispatch t event) ~f:(function
              | "header" -> assert h
              | "footer" -> assert f
              | _ -> assert false));
          assert (List.is_empty (commit t view)))));
  let update = Reconciler.prepare t ~theme:Theme.default None |> ok in
  Reconciler.accept t update |> ok;
  List.iter (body_event :: !stale_actions) ~f:(fun event ->
    assert (Option.is_none (Reconciler.dispatch t event)));
  print_endline
    "default Card unchanged; all variants/themes/slots keep checked body and latest \
     action; removed slots and full unmount fence actions; repeats are idle";
  [%expect
    {| default Card unchanged; all variants/themes/slots keep checked body and latest action; removed slots and full unmount fence actions; repeats are idle |}]
;;

let%expect_test "status regions preserve surviving actions across slot changes" =
  let t = Reconciler.create window in
  let bar p leading center trailing =
    let control name = View.button name ~on_click:(fun () -> name) in
    P.status_bar
      p
      ?leading:(Option.some_if leading (control "leading"))
      ?center:(Option.some_if center (control "center"))
      ?trailing:(Option.some_if trailing (control "trailing"))
      ()
  in
  let previous = ref [] in
  List.iter [ P.Appearance.light; P.Appearance.dark ] ~f:(fun p ->
    List.iter
      [ true, true, true
      ; false, true, true
      ; false, true, false
      ; true, true, false
      ; true, false, false
      ; true, false, true
      ; false, false, true
      ; false, false, false
      ; true, true, true
      ]
      ~f:(fun (leading, center, trailing) ->
        let view = bar p leading center trailing in
        let operations = commit t view in
        let survivors =
          List.filter !previous ~f:(fun (_, _, name) ->
            match name with
            | "leading" -> leading
            | "center" -> center
            | "trailing" -> trailing
            | _ -> assert false)
        in
        List.iter !previous ~f:(fun (node, handler, name) ->
          let survives =
            List.exists survivors ~f:(fun (_, _, other) -> String.equal name other)
          in
          let action =
            Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L))
          in
          assert (Option.equal String.equal action (Option.some_if survives name)));
        let added =
          List.filter_map operations ~f:(function
            | W.Op.Create (node, Button, _, Some handler) ->
              let name =
                Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L))
                |> Option.value_exn
              in
              Some (node, handler, name)
            | _ -> None)
        in
        previous := survivors @ added;
        assert (
          List.length !previous
          = Bool.to_int leading + Bool.to_int center + Bool.to_int trailing);
        assert (List.is_empty (commit t view))));
  print_endline
    "all eight slot combinations in both themes; retained actions, retired fences, idle \
     repeats";
  [%expect
    {| all eight slot combinations in both themes; retained actions, retired fences, idle repeats |}]
;;

let%expect_test "links retain actions and disabled fencing; shortcuts never bind actions" =
  let t = Reconciler.create window in
  let link ?(disabled = false) action =
    P.link P.Appearance.light ~disabled ~on_click:(fun () -> action) "Documentation"
  in
  let initial = commit t (link `First) in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let event = W.Event.Press (window, node, handler, 1L) in
  assert (List.is_empty (commit t (link `Latest)));
  (match Reconciler.dispatch t event with
   | Some `Latest -> ()
   | Some `First | None -> assert false);
  ignore (commit t (link ~disabled:true `Latest) : W.Op.t list);
  assert (Option.is_none (Reconciler.dispatch t event));
  let d = View.Expert.describe (link `Latest) in
  let metadata = Option.value_exn d.accessibility |> Accessibility.Expert.to_wire in
  assert (
    Option.equal Gpuio_protocol.Accessibility_wire.Role.equal metadata.role (Some Link));
  let shortcut = P.shortcut_label P.Appearance.light [ "⌘"; "K" ] in
  List.iter (descriptions shortcut) ~f:(fun d ->
    assert (Option.is_none d.on_click && Option.is_none d.commands));
  print_endline
    "native Link role, latest action, disabled rejection, display-only shortcut";
  [%expect
    {| native Link role, latest action, disabled rejection, display-only shortcut |}]
;;

let%expect_test
    "description semantics and explicit announcement priorities survive layout"
  =
  let role view =
    let d = View.Expert.describe view in
    Option.value_exn d.accessibility |> Accessibility.Expert.to_wire
  in
  let items =
    [ P.Description.create
        ~key:(Key.of_int 1)
        ~term:"Model"
        ~definition:(View.text "Local")
    ]
  in
  List.iter [ false; true ] ~f:(fun stacked ->
    let view = P.description_list P.Appearance.light ~stacked items in
    assert (
      Option.equal
        Gpuio_protocol.Accessibility_wire.Role.equal
        (role view).role
        (Some Description_list));
    let entry =
      List.hd_exn (View.Expert.describe view).children |> View.Expert.describe
    in
    let roles = List.map entry.children ~f:(fun view -> (role view).role) in
    assert (
      List.equal
        (Option.equal Gpuio_protocol.Accessibility_wire.Role.equal)
        roles
        [ Some Term; Some Definition ]));
  List.iter [ Accessibility.Live.Off; Polite; Assertive ] ~f:(fun live ->
    let config = role (P.alert P.Appearance.light ~live ~title:"Status" []) in
    print_s [%sexp (config.live : Gpuio_protocol.Accessibility_wire.Live.t)]);
  [%expect
    {|
    Off
    Polite
    Assertive
  |}]
;;

let%expect_test "overlay badge counts and labels are validated before rendering" =
  List.iter
    [ -1, 99; 1, -1 ]
    ~f:(fun (count, max) ->
      assert (Or_error.is_error (P.Overlay_badge.count ~max ~label:"Unread" count)));
  List.iter
    [ ""; " \t"; "bad\000label"; "\255"; String.make 4097 'x' ]
    ~f:(fun label ->
      assert (Or_error.is_error (P.Overlay_badge.count ~label 1));
      assert (Or_error.is_error (P.Overlay_badge.dot ~label)));
  List.iter
    [ 0, 99, ""; 7, 99, "7"; 150, 99, "99+"; 1, 0, "0+"; Int.max_value, 99, "99+" ]
    ~f:(fun (count, max, visible) ->
      let label = sprintf "%d unread messages 世界" count in
      let badge = P.Overlay_badge.count ~max ~label count |> ok in
      let ds =
        descriptions (P.overlay_badge P.Appearance.dark ~badge (View.text "Body"))
      in
      let text =
        List.filter_map ds ~f:(fun d ->
          if View.Expert.Kind.equal d.kind Text && not (String.equal d.text "Body")
          then Some d.text
          else None)
      in
      assert (List.equal String.equal text (if count = 0 then [] else [ visible ]));
      let labels =
        List.filter_map ds ~f:(fun d ->
          Option.bind d.accessibility ~f:(fun a -> (Accessibility.Expert.to_wire a).label))
      in
      assert (List.equal String.equal labels (if count = 0 then [] else [ label ])));
  print_endline
    "negative values and malformed labels rejected; zero omitted; capped visual with \
     uncapped Unicode label; max_int safe";
  [%expect
    {| negative values and malformed labels rejected; zero omitted; capped visual with uncapped Unicode label; max_int safe |}]
;;

let%expect_test "overlay changes keep the underlying control and dispose only decorations"
  =
  let owner = Asset.Expert.Owner.create () in
  let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let asset = Asset.Expert.handle ~owner ~id ~format:Svg in
  let icon =
    Icon.Config.create ~asset ~description:Image.Description.decorative () |> ok
  in
  let t = Reconciler.create ~asset_owner:owner window in
  let count value =
    P.Overlay_badge.count ~label:(sprintf "%d unread" value) value |> ok
  in
  let view appearance size badge action =
    P.overlay_badge
      appearance
      ~size
      ~badge
      (View.button "Inbox" ~on_click:(fun () -> action))
  in
  let initial = commit t (view P.Appearance.light Medium (count 7) `First) in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  List.iter [ P.Appearance.light; P.Appearance.dark ] ~f:(fun appearance ->
    List.iter [ P.Size.Small; Medium; Large ] ~f:(fun size ->
      List.iter
        [ count 150
        ; P.Overlay_badge.dot ~label:"New activity" |> ok
        ; P.Overlay_badge.icon icon
        ; count 0
        ; count 7
        ]
        ~f:(fun badge ->
          let current = view appearance size badge `Latest in
          let operations = commit t current in
          List.iter operations ~f:(function
            | W.Op.Remove removed ->
              assert (not (Gpuio_protocol.Node_id.equal node removed))
            | Create (_, Button, _, _) -> failwith "badged control was replaced"
            | _ -> ());
          (match Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L)) with
           | Some `Latest -> ()
           | Some `First | None -> assert false);
          assert (List.is_empty (commit t current)))));
  let removal = Reconciler.prepare t ~theme:Theme.default None |> ok in
  Reconciler.accept t removal |> ok;
  assert (
    Option.is_none (Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L))));
  print_endline
    "count/dot/icon/zero, both appearances and all sizes retain body identity and latest \
     action; unmount fences delivery";
  [%expect
    {| count/dot/icon/zero, both appearances and all sizes retain body identity and latest action; unmount fences delivery |}]
;;

let%expect_test "presentation capability includes the accepted native family" =
  assert (Int64.equal (Int64.bit_and W.capabilities 17179869184L) 17179869184L);
  let bytes = W.Message.encode (Hello (W.version, W.capabilities)) |> ok in
  String.iter bytes ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0001fcffffffffffff0000 |}]
;;

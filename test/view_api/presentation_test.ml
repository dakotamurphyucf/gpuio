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

let%expect_test "presentation capability includes the accepted native family" =
  assert (Int64.equal (Int64.bit_and W.capabilities 17179869184L) 17179869184L);
  let bytes = W.Message.encode (Hello (W.version, W.capabilities)) |> ok in
  String.iter bytes ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0001fcffffffff7f000000 |}]
;;

open Core
open Gpuio
module P = Presentation
module A = P.Attachment
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let describe = View.Expert.describe
let style = Style.create_exn
let px = Length.px_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let empty_theme = Theme.create [] |> ok
let statuses = [ A.Status.Pending; Uploading; Processing; Failed; Complete ]

let commit ?(theme = Theme.default) t view =
  let update = Reconciler.prepare t ~theme view |> ok in
  Reconciler.accept t update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let fields ?(theme = empty_theme) view =
  Style.Expert.to_wire (describe view).style ~theme
  |> ok
  |> List.concat_map ~f:(function
    | W.Style.Fields fields -> fields
    | State _ -> []
    | _ -> failwith "unexpected legacy style encoding")
;;

let has view field = List.mem (fields view) field ~equal:W.Field.equal

let child view name =
  List.find_exn (describe view).children ~f:(fun child ->
    Option.equal Key.equal (describe child).key (Some (key name)))
;;

let title ?status ?shimmer name =
  A.Title.create ~key:(key name) ?status ?shimmer name |> ok |> A.Content.Item.title
;;

let content ?status ?shimmer () =
  A.Content.create
    [ title ?status ?shimmer "Résumé 👩🏽‍💻.png"
    ; A.Content.Item.description (A.Description.create ~key:(key "detail") "2.4 MB")
    ]
;;

let trigger ?disabled revision =
  A.Trigger.create
    ~key:(key "open")
    ~accessible_name:"Open attachment"
    ?disabled
    ~on_click:(fun () -> "open:" ^ revision)
    ()
  |> ok
;;

let%expect_test "slot removal retires only its owner and appearance needs no theme tokens"
  =
  let t = Reconciler.create window in
  let make ~media ~trigger_enabled ~actions revision =
    A.create
      P.Appearance.light
      ?media:(Option.some_if media (A.Media.create [ View.text "PDF" ]))
      ~content:(A.Content.create [ title "Report.pdf" ])
      ?trigger:(Option.some_if trigger_enabled (trigger revision))
      ?actions:
        (Option.some_if
           actions
           (A.Actions.create
              [ View.checkbox
                  ~key:(key "keep")
                  ~state:Checked
                  ~on_toggle:(fun () -> "keep:" ^ revision)
                  "Keep offline"
              ]))
      ()
  in
  let first =
    commit t (Some (make ~media:true ~trigger_enabled:true ~actions:true "old"))
  in
  let action kind =
    List.find_map_exn first ~f:(function
      | W.Op.Create (node, found, _, Some handler) when W.Kind.equal kind found ->
        Some (node, W.Event.Press (window, node, handler, 1L))
      | _ -> None)
  in
  let trigger_node, trigger_event = action Button in
  let keep_node, keep_event = action Checkbox in
  let removed =
    commit t (Some (make ~media:false ~trigger_enabled:false ~actions:true "new"))
  in
  assert (
    List.exists removed ~f:(function
      | W.Op.Remove node -> Gpuio_protocol.Node_id.equal node trigger_node
      | _ -> false));
  assert (
    not
      (List.exists removed ~f:(function
         | W.Op.Remove node -> Gpuio_protocol.Node_id.equal node keep_node
         | _ -> false)));
  assert (Option.is_none (Reconciler.dispatch t trigger_event));
  assert (Option.equal String.equal (Reconciler.dispatch t keep_event) (Some "keep:new"));
  ignore
    (commit t (Some (make ~media:true ~trigger_enabled:true ~actions:false "last"))
     : W.Op.t list);
  assert (Option.is_none (Reconciler.dispatch t keep_event));
  assert (Option.is_none (Reconciler.dispatch t trigger_event));
  ignore (commit t None : W.Op.t list);
  let view =
    A.create
      P.Appearance.dark
      ~status:Failed
      ~trigger:(trigger "concrete")
      ~media:(A.Media.create [])
      ~content:(content ())
      ()
  in
  ignore (commit ~theme:empty_theme t (Some view) : W.Op.t list);
  assert (List.is_empty (commit ~theme:empty_theme t (Some view)));
  Reconciler.close t;
  print_endline
    "surviving checkbox keeps state/current callback; removed and remounted trigger \
     rejects old events; empty theme works";
  [%expect
    {| surviving checkbox keeps state/current callback; removed and remounted trigger rejects old events; empty theme works |}]
;;

let%expect_test "attachment inputs validate future loading and accessible activation" =
  List.iter [ Float.nan; Float.infinity; 0.; 0.99; 1_000_001. ] ~f:(fun value ->
    assert (Result.is_error (A.Size.pixels value)));
  List.iter [ 1.; 40.; 1_000_000. ] ~f:(fun value ->
    ignore (A.Size.pixels value |> ok : A.Size.t));
  List.iter
    [ "\255"; String.make 16_385 'x' ]
    ~f:(fun text -> assert (Result.is_error (A.Title.create ~key:(key "title") text)));
  List.iter
    [ ""; String.make 16_384 'x'; "مرحبا · 👩🏽‍💻" ]
    ~f:(fun text -> ignore (A.Title.create ~key:(key "title") text |> ok : A.Title.t));
  List.iter
    [ ""; "\255"; "bad\000name"; String.make 1025 'x' ]
    ~f:(fun accessible_name ->
      assert (
        Result.is_error
          (A.Trigger.create ~key:(key "open") ~accessible_name ~on_click:Fn.id ())));
  print_endline
    "size, UTF-8, byte boundaries and trigger names validated before submission";
  [%expect
    {| size, UTF-8, byte boundaries and trigger names validated before submission |}]
;;

let%expect_test "status, size and axis transitions preserve actions and source identity" =
  let t = Reconciler.create window in
  let initial_nodes = ref [] in
  let events = ref [] in
  let count = ref 0 in
  List.iter [ P.Appearance.light; P.Appearance.dark ] ~f:(fun appearance ->
    List.iter [ P.Axis.Horizontal; Vertical ] ~f:(fun axis ->
      List.iter
        [ A.Size.xsmall
        ; A.Size.small
        ; A.Size.medium
        ; A.Size.large
        ; A.Size.pixels 56. |> ok
        ]
        ~f:(fun size ->
          List.iter statuses ~f:(fun status ->
            incr count;
            let revision = Int.to_string !count in
            let view =
              A.create
                appearance
                ~axis
                ~size
                ~status
                ~content:(content ())
                ~media:(A.Media.create [ View.text "PDF" ])
                ~trigger:(trigger revision)
                ~actions:
                  (A.Actions.create
                     [ View.button ~key:(key "remove") "Remove" ~on_click:(fun () ->
                         "remove:" ^ revision)
                     ])
                ()
            in
            let title = child (child view "content") "Résumé 👩🏽‍💻.png" in
            assert (
              Bool.equal
                (Option.is_some (describe title).text_shimmer)
                (A.Status.is_in_progress status));
            assert (
              has view (Border_style (if A.Status.equal status Pending then 1L else 0L)));
            let media = child view "media" in
            (match axis with
             | Horizontal -> assert (not (has media (Aspect_ratio 1.)))
             | Vertical ->
               assert (has media (Aspect_ratio 1.) && has media (Width (Percent 100.))));
            assert (has (child view "actions") (Pointer_occlusion 1L));
            let operations = commit t (Some view) in
            if !count = 1
            then (
              initial_nodes
              := List.filter_map operations ~f:(function
                   | W.Op.Create (node, _, _, _) -> Some node
                   | _ -> None);
              events
              := List.filter_map operations ~f:(function
                   | W.Op.Create (node, Button, _, Some handler) ->
                     Some (W.Event.Press (window, node, handler, 1L))
                   | _ -> None);
              assert (List.length !events = 2))
            else
              List.iter operations ~f:(function
                | W.Op.Create _ | Remove _ ->
                  failwith "status/layout update replaced an owner"
                | _ -> ());
            let dispatched =
              List.filter_map !events ~f:(Reconciler.dispatch t)
              |> List.sort ~compare:String.compare
            in
            assert (
              [%equal: string list]
                dispatched
                [ "open:" ^ revision; "remove:" ^ revision ]);
            assert (List.is_empty (commit t (Some view)))))));
  ignore (commit t None : W.Op.t list);
  List.iter !events ~f:(fun event ->
    assert (Option.is_none (Reconciler.dispatch t event)));
  assert (not (List.is_empty !initial_nodes));
  printf
    "%d cases: source/action owners retained, latest callbacks delivered, repeated views \
     idle, unmount fences events\n"
    !count;
  [%expect
    {| 100 cases: source/action owners retained, latest callbacks delivered, repeated views idle, unmount fences events |}]
;;

let%expect_test "explicit title status and shimmer overrides remain independent" =
  let root = Text_shimmer.Config.create ~direction:Right_to_left () |> ok in
  let own = Text_shimmer.Config.create ~animated:false () |> ok in
  let appearance = P.Appearance.with_text_shimmer P.Appearance.light own in
  let config view =
    (describe (child (child view "content") "Résumé 👩🏽‍💻.png")).text_shimmer
  in
  let make ?shimmer ?title_status ?title_shimmer status =
    A.create
      appearance
      ~status
      ?shimmer
      ~content:(content ?status:title_status ?shimmer:title_shimmer ())
      ()
  in
  assert (Option.equal Text_shimmer.Config.equal (config (make Uploading)) (Some own));
  assert (
    Option.equal
      Text_shimmer.Config.equal
      (config (make ~shimmer:root Processing))
      (Some root));
  assert (
    Option.equal
      Text_shimmer.Config.equal
      (config (make ~shimmer:root ~title_shimmer:own ~title_status:Uploading Complete))
      (Some own));
  assert (Option.is_none (config (make ~title_status:Failed Uploading)));
  let light = A.create P.Appearance.light ~status:Uploading ~content:(content ()) () in
  let dark = A.create P.Appearance.dark ~status:Uploading ~content:(content ()) () in
  assert (not (Option.equal Text_shimmer.Config.equal (config light) (config dark)));
  print_endline
    "appearance -> card -> title config; independent title status; built-in themes differ";
  [%expect
    {| appearance -> card -> title config; independent title status; built-in themes differ |}]
;;

let%expect_test "slot styles refine defaults while action shielding remains invariant" =
  let custom =
    style [ Pointer_occlusion None; Padding_left (px 23.) ]
    |> fun s -> Style.with_state_exn s Hovered [ Opacity 0.9 ]
  in
  assert (
    Result.is_error (Style.with_state Style.empty Hovered [ Pointer_occlusion None ]));
  assert (
    Result.is_error
      (Style.with_state Style.empty Pressed [ Pointer_occlusion Pointer_and_scroll ]));
  let view =
    A.create
      P.Appearance.light
      ~status:Failed
      ~style:(style [ Border_color (Color.rgb_exn 0x123456) ])
      ~content:
        (A.Content.create
           [ A.Content.Item.description
               (A.Description.create ~key:(key "failed") "Failed")
           ; A.Content.Item.description
               (A.Description.create ~key:(key "normal") ~status:Complete "Normal")
           ; A.Content.Item.description
               (A.Description.create
                  ~key:(key "custom")
                  ~style:(style [ Foreground (Color.rgb_exn 0xabcdef) ])
                  "Custom")
           ])
      ~actions:(A.Actions.create ~style:custom [ View.text "Action area" ])
      ()
  in
  assert (has view (Border_color (Rgba 0x123456ffL)));
  let body = child view "content" in
  assert (has (child body "failed") (Foreground (Rgba 0xb7344bccL)));
  assert (has (child body "normal") (Foreground (Rgba 0x606a79ffL)));
  assert (has (child body "custom") (Foreground (Rgba 0xabcdefffL)));
  let actions = child view "actions" in
  assert (has actions (Padding_left (Px 23.)));
  assert (has actions (Pointer_occlusion 1L));
  let styles = Style.Expert.to_wire (describe actions).style ~theme:empty_theme |> ok in
  List.iter styles ~f:(function
    | W.Style.State (_, fields) ->
      assert (
        not
          (List.exists fields ~f:(function
             | Pointer_occlusion _ -> true
             | _ -> false)))
    | _ -> ());
  assert (
    List.exists styles ~f:(function
      | W.Style.State (_, fields) -> List.mem fields (Opacity 0.9) ~equal:W.Field.equal
      | _ -> false));
  let t = Reconciler.create window in
  ignore (commit t (Some view) : W.Op.t list);
  assert (List.is_empty (commit t (Some view)));
  Reconciler.close t;
  print_endline
    "root/description overrides and status inheritance; base/state custom styles cannot \
     remove pointer shielding";
  [%expect
    {| root/description overrides and status inheritance; base/state custom styles cannot remove pointer shielding |}]
;;

let%expect_test
    "image opacity excludes overlays and only absent images use failed media colors"
  =
  let owner = Asset.Expert.Owner.create () in
  let asset =
    Asset.Expert.handle
      ~owner
      ~id:(Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok)
      ~format:Png
  in
  let image = A.Media.Image.create ~asset ~description:Image.Description.decorative () in
  let make ?image status =
    A.create
      P.Appearance.light
      ~status
      ~media:(A.Media.create ?image ~overlay:(View.text "Overlay") [ View.text "Child" ])
      ()
  in
  List.iter statuses ~f:(fun status ->
    let media = child (make ~image status) "media" in
    let image_view = child media "image" in
    let config = (Option.value_exn (describe image_view).image).config in
    assert (Image.Fit.equal (Image.Config.fit config) Cover);
    assert (
      has
        image_view
        (Opacity
           (if A.Status.is_in_progress status || A.Status.equal status Failed
            then 0.6
            else 1.)));
    List.iter
      [ media; child media "overlay"; child media "children" ]
      ~f:(fun view ->
        assert (
          not
            (List.exists (fields view) ~f:(function
               | Opacity _ -> true
               | _ -> false)))));
  assert (has (child (make Failed) "media") (Background (Solid (Rgba 0xb7344b1aL))));
  assert (has (child (make ~image Failed) "media") (Background (Solid (Rgba 0xf0f2f6ffL))));
  let t = Reconciler.create ~asset_owner:owner window in
  ignore (commit t (Some (make ~image Uploading)) : W.Op.t list);
  assert (List.is_empty (commit t (Some (make ~image Uploading))));
  ignore (commit t None : W.Op.t list);
  print_endline
    "image Cover; work/failure alpha 0.6; overlay/children undimmed; no-image failure \
     alpha 0.1";
  [%expect
    {| image Cover; work/failure alpha 0.6; overlay/children undimmed; no-image failure alpha 0.1 |}]
;;

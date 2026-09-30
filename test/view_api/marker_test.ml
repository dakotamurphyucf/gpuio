open Core
open Gpuio
module M = Presentation.Marker
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let appearance = Presentation.Appearance.dark
let text ?style value = M.Content.Item.text ~key:(key "text") ?style value |> ok

let content ?style items =
  M.Content.create ~key:(key "content") ?style items |> ok |> M.Item.content
;;

let create ?variant ?loading ?loading_style ?shimmer ?spinner items =
  M.create
    appearance
    ~key:(key "marker")
    ?variant
    ?loading
    ?loading_style
    ?shimmer
    ?spinner
    items
  |> ok
;;

let rec nodes view =
  let description = View.Expert.describe view in
  description :: List.concat_map description.children ~f:nodes
;;

let programs view =
  List.filter_map (nodes view) ~f:(fun d ->
    Option.map d.animation_program ~f:(fun p ->
      Animation.Expert.program_to_wire p.config ~generation:1L |> ok))
;;

let commit t view =
  let update = Reconciler.prepare t ~theme:Theme.default view |> ok in
  Reconciler.accept t update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test "marker validates text, identities and localized spinner configuration" =
  List.iter
    [ ""; "é · 👩‍💻 · العربية"; String.make 16384 'x' ]
    ~f:(fun value -> ignore (text value));
  List.iter
    [ "\255"; String.make 16385 'x' ]
    ~f:(fun value -> assert (Result.is_error (M.Content.Item.text ~key:(key "t") value)));
  assert (Result.is_error (M.Content.create ~key:(key "content") [ text "a"; text "b" ]));
  let icon = M.Icon.create ~key:(key "content") [] |> M.Item.icon in
  assert (Result.is_error (M.create appearance [ icon; content [] ]));
  assert (
    Result.is_error
      (M.create
         appearance
         [ M.Item.element ~key:(key "gpuio:marker:before") (View.text "x") ]));
  List.iter
    [ ""; "\000"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (M.Spinner.create ~label ())));
  let spinner = M.Spinner.create ~label:"正在同步" ~animated:false () |> ok in
  let view = create ~loading:true ~spinner [] in
  let loading =
    List.find_map_exn (nodes view) ~f:(fun n -> n.loading) |> Loading.Expert.to_wire
  in
  assert (String.equal loading.label "正在同步");
  assert (not loading.animated);
  print_endline
    "bounded UTF-8; duplicate/reserved keys rejected; validated localized static spinner";
  [%expect
    {| bounded UTF-8; duplicate/reserved keys rejected; validated localized static spinner |}]
;;

let%expect_test
    "typed icon suppresses spinner; typed text alone controls shimmer vs pulse"
  =
  let rich = M.Content.Item.element ~key:(key "rich") (View.text "rich") in
  let arbitrary = M.Item.element ~key:(key "arbitrary") (View.text "icon-shaped") in
  let empty_icon = M.Icon.create ~key:(key "icon") [] |> M.Item.icon in
  let spinners view = List.count (nodes view) ~f:(fun n -> Option.is_some n.loading) in
  assert (spinners (create ~loading:true [ arbitrary; content [ rich ] ]) = 1);
  assert (spinners (create ~loading:true [ empty_icon; content [ rich ] ]) = 0);
  List.iter
    [ []; [ rich ]; [ text "" ]; [ text "Thinking" ]; [ rich; text "Thinking" ] ]
    ~f:(fun items ->
      let view =
        create ~loading:true ~loading_style:Shimmer [ content items; arbitrary ]
      in
      assert (spinners view = 0);
      let p = List.hd_exn (programs view) in
      let has_text =
        List.exists (nodes view) ~f:(fun n -> Option.is_some n.text_shimmer)
      in
      assert (W.Animation.Repeat.equal p.program.repeat (if has_text then Once else Loop));
      List.iter (nodes view) ~f:(fun n ->
        if String.equal n.text "rich" || String.equal n.text "icon-shaped"
        then assert (Option.is_none n.text_shimmer));
      printf
        "%s\n"
        (if has_text
         then "typed text shimmers; rich siblings static"
         else "rich-only content pulses"));
  [%expect
    {|
    rich-only content pulses
    rich-only content pulses
    typed text shimmers; rich siblings static
    typed text shimmers; rich siblings static
    typed text shimmers; rich siblings static
    |}]
;;

let%expect_test
    "pulse duration is bounded and exact; stopped content restores styled opacity"
  =
  List.iter [ 1; 3; 2000; 60000 ] ~f:(fun duration ->
    List.iter [ Text_shimmer.Repeat.Once; Loop ] ~f:(fun repeat ->
      let shimmer =
        Text_shimmer.Config.create
          ~duration:(Time_ns.Span.of_ms (Float.of_int duration))
          ~repeat
          ()
        |> ok
      in
      let view = create ~loading:true ~loading_style:Shimmer ~shimmer [ content [] ] in
      let p = (List.hd_exn (programs view)).program in
      let total =
        List.sum
          (module Int64)
          p.stages
          ~f:(fun stage ->
            match stage.W.Animation_program.Stage.timing with
            | Tween (ms, _) -> ms
            | Spring _ -> assert false)
      in
      assert (Int64.equal total (Int64.of_int duration));
      let values =
        List.map p.stages ~f:(fun s ->
          (List.hd_exn s.W.Animation_program.Stage.targets).value)
      in
      assert (List.equal Float.equal values [ 0.6; 1. ])));
  let custom =
    Style.create_exn [ Opacity 0.4; Grow 1. ]
    |> fun s -> Style.with_state_exn s Hovered [ Opacity 0.2 ]
  in
  List.iter [ true; false ] ~f:(fun loading ->
    let shimmer = Text_shimmer.Config.create ~animated:false () |> ok in
    let view =
      create ~loading ~loading_style:Shimmer ~shimmer [ content ~style:custom [] ]
    in
    let p = (List.hd_exn (programs view)).program in
    assert (W.Animation.Repeat.equal p.repeat Once);
    let stage = List.hd_exn p.stages in
    assert (W.Animation_program.Timing.equal stage.timing (Tween (0L, Ease_in_out)));
    assert (Float.equal (List.hd_exn stage.targets).value 1.);
    let n = List.find_exn (nodes view) ~f:(fun n -> Option.is_some n.animation_program) in
    let fields = Style.Expert.to_wire n.style ~theme:Theme.default |> ok in
    assert (
      List.exists fields ~f:(function
        | State (2L, fields) -> List.mem fields (Opacity 0.2) ~equal:W.Field.equal
        | _ -> false)));
  print_endline
    "1ms/odd/full durations; Once/Loop; disabled and stopped restore factor one with \
     state styles intact";
  [%expect
    {| 1ms/odd/full durations; Once/Loop; disabled and stopped restore factor one with state styles intact |}]
;;

let%expect_test
    "loading, layout and content changes preserve direct rich controls and current \
     callbacks"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let view revision ~variant ~loading ~loading_style ~typed ~icon ~reversed =
    let rich =
      M.Content.Item.element
        ~key:(key "button")
        (View.button
           ~key:(key "old-view-key")
           ~style:(Style.create_exn [ Grow 1. ])
           ~on_click:(fun () -> revision)
           "Keep me")
    in
    let items = rich :: (if typed then [ text "Thinking" ] else []) in
    let items = if reversed then List.rev items else items in
    create
      ~variant
      ~loading
      ~loading_style
      ((if icon then [ M.Item.icon (M.Icon.create ~key:(key "icon") []) ] else [])
       @ [ content items ])
  in
  let initial =
    view
      0
      ~variant:Plain
      ~loading:false
      ~loading_style:Spinner
      ~typed:false
      ~icon:false
      ~reversed:false
  in
  let node, handler =
    List.find_map_exn (commit reconciler (Some initial)) ~f:(function
      | W.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let count = ref 0 in
  List.iter [ M.Variant.Plain; Separator; Border ] ~f:(fun variant ->
    List.iter [ false; true ] ~f:(fun loading ->
      List.iter [ M.Loading_style.Spinner; Shimmer ] ~f:(fun loading_style ->
        List.iter [ false; true ] ~f:(fun typed ->
          incr count;
          let next =
            view
              !count
              ~variant
              ~loading
              ~loading_style
              ~typed
              ~icon:Int.(!count % 2 = 0)
              ~reversed:true
          in
          let ops = commit reconciler (Some next) in
          List.iter ops ~f:(function
            | W.Op.Create (_, Button, _, _) -> failwith "control remounted"
            | Remove removed -> assert (not (Gpuio_protocol.Node_id.equal removed node))
            | _ -> ());
          assert (
            Option.equal
              Int.equal
              (Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L)))
              (Some !count));
          assert (List.is_empty (commit reconciler (Some next)))))));
  ignore (commit reconciler None : W.Op.t list);
  assert (
    Option.is_none
      (Reconciler.dispatch reconciler (W.Event.Press (window, node, handler, 1L))));
  printf
    "%d transitions; retained control/current callback; equal snapshots idle; retired \
     events rejected\n"
    !count;
  [%expect
    {| 24 transitions; retained control/current callback; equal snapshots idle; retired events rejected |}]
;;

let%expect_test "marker styles refine slots directly without wrapping rich layout" =
  let custom_line = Style.create_exn [ Height (Length.px_exn 3.) ] in
  let rich =
    View.button
      ~key:(key "discarded")
      ~style:(Style.create_exn [ Grow 1. ])
      ~on_click:(fun () -> ())
      "Direct"
  in
  let view =
    M.create
      appearance
      ~variant:Separator
      ~separator_style:custom_line
      ~style:(Style.create_exn [ Gap (Length.px_exn 4.); Font_size 18. ])
      [ content
          ~style:(Style.create_exn [ Grow 1.; Shrink 1.; Text_align Left ])
          [ M.Content.Item.element ~key:(key "direct") rich ]
      ]
    |> ok
  in
  let root = View.Expert.describe view in
  let fields d =
    Style.Expert.to_wire d.View.Expert.style ~theme:Theme.default
    |> ok
    |> List.concat_map ~f:(function
      | W.Style.Fields fields -> fields
      | _ -> [])
  in
  assert (List.mem (fields root) (Font_size 18.) ~equal:W.Field.equal);
  (match root.children with
   | [ before; body; after ] ->
     List.iter [ before; after ] ~f:(fun line ->
       let line = View.Expert.describe line in
       assert (List.mem (fields line) (Height (Px 3.)) ~equal:W.Field.equal));
     let body = View.Expert.describe body in
     assert (List.mem (fields body) (Grow 1.) ~equal:W.Field.equal);
     assert (List.mem (fields body) (Shrink 1.) ~equal:W.Field.equal);
     assert (List.mem (fields body) (Text_align 0L) ~equal:W.Field.equal);
     let direct = View.Expert.describe (List.hd_exn body.children) in
     assert (View.Expert.Kind.equal direct.kind Button);
     assert (Option.equal Key.equal direct.key (Some (key "direct")));
     assert (List.mem (fields direct) (Grow 1.) ~equal:W.Field.equal)
   | _ -> assert false);
  let metadata =
    Accessibility.create ~role:Status ~live:Off ~label:"Localized status" () |> ok
  in
  let annotated = View.with_accessibility view metadata |> ok |> View.Expert.describe in
  assert (Option.is_some annotated.accessibility);
  assert (Option.is_none root.accessibility);
  print_endline
    "root/line/content overrides; rich control remains direct flex child; explicit \
     status only";
  [%expect
    {| root/line/content overrides; rich control remains direct flex child; explicit status only |}]
;;

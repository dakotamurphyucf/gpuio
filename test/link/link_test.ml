open Core
open Gpuio
module W = Gpuio_protocol.Link_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let commit t view =
  let update = Reconciler.prepare t ~theme:Theme.default view |> ok in
  Reconciler.accept t update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test "composed links require a distinct paired host capability" =
  let capability = Int64.shift_left 1L 48 in
  assert (Int64.equal (Int64.bit_and Wire.capabilities capability) capability);
  List.iter [ capability; Wire.capabilities ] ~f:(fun required ->
    let hello = Wire.Message.Hello (Wire.version, required) in
    Wire.Message.encode hello
    |> ok
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline "");
  [%expect
    {|
    0001fc0000000000000100
    0001fcffffffffffff0100
    |}]
;;

let%expect_test "link operation and kind match independent Rust bytes" =
  let config =
    Link.Config.create ~label:"Guide 世界" ~tab_stop:false ~tab_index:(-2) () |> ok
  in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let print operations =
    let message = Wire.Message.Apply { window; base = 0L; revision = 1L; operations } in
    let bytes = Bin_prot.Utils.bin_dump Wire.Message.bin_writer_t message in
    Bigstring.to_string bytes
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline ""
  in
  print [ Set_link (node, Link.Expert.to_wire config) ];
  print [ Create (node, Link, "", None) ];
  [%expect
    {|
    0300010001013c00010c477569646520e4b896e7958c0000fffe
    030001000101000001330000
    |}]
;;

let%expect_test "composed link updates retain identity and use current actions" =
  let t = Reconciler.create window in
  let view ?(disabled = false) ?(tab_stop = true) ?(tab_index = 0) text action =
    let config =
      Link.Config.create ~label:"Read guide" ~disabled ~tab_stop ~tab_index () |> ok
    in
    View.link
      ~key:(Key.of_string "guide" |> ok)
      config
      ~on_click:(fun () -> action)
      [ View.column [ View.text text; View.text "Secondary detail" ] ]
    |> ok
  in
  let initial = commit t (Some (view "Original" "first")) in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (node, Link, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let event = Wire.Event.Press (window, node, handler, 1L) in
  assert (Option.equal String.equal (Reconciler.dispatch t event) (Some "first"));
  let changed = view ~tab_stop:false ~tab_index:(-3) "Changed 世界" "latest" in
  let operations = commit t (Some changed) in
  List.iter operations ~f:(function
    | Wire.Op.Create _ | Remove _ -> failwith "content/config update replaced nodes"
    | _ -> ());
  assert (
    List.exists operations ~f:(function
      | Wire.Op.Set_link _ -> true
      | _ -> false));
  assert (Option.equal String.equal (Reconciler.dispatch t event) (Some "latest"));
  assert (List.is_empty (commit t (Some changed)));
  ignore (commit t (Some (view ~disabled:true "Disabled" "disabled")) : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch t event));
  let operations = commit t (Some (view "Enabled" "re-enabled")) in
  let next_handler =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (Option.is_none (Reconciler.dispatch t event));
  let event = Wire.Event.Press (window, node, next_handler, 4L) in
  assert (Option.equal String.equal (Reconciler.dispatch t event) (Some "re-enabled"));
  ignore (commit t None : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch t event));
  print_endline
    "retained root; current callback; idle repeats; disabled and retired actions fenced";
  [%expect
    {| retained root; current callback; idle repeats; disabled and retired actions fenced |}]
;;

let%expect_test
    "composed content admits passive layout but rejects nested input and shields"
  =
  let config = Link.Config.create ~label:"Guide" () |> ok in
  let link children = View.link config ~on_click:(fun () -> ()) children in
  let text style = View.text ~style "content" in
  let forbidden =
    [ Style.Property.User_select true
    ; Inert true
    ; Overflow_x Scroll
    ; Overflow_y Scroll
    ; Pointer_occlusion Pointer
    ; Pointer_occlusion Pointer_and_scroll
    ]
  in
  List.iter forbidden ~f:(fun property ->
    let style = Style.create_exn [ property ] in
    assert (Or_error.is_error (link [ View.column [ text style ] ])));
  List.iter [ Style.Property.Overflow_x Scroll; Overflow_y Scroll ] ~f:(fun property ->
    let style = Style.with_state_exn Style.empty Hovered [ property ] in
    assert (Or_error.is_error (link [ View.column [ text style ] ])));
  assert (Or_error.is_error (link [ View.button "Nested" ~on_click:(fun () -> ()) ]));
  assert (Or_error.is_error (link [ link [] |> ok ]));
  ignore
    (link
       [ View.row
           [ text
               (Style.create_exn
                  [ User_select false; Overflow_x Hidden; Overflow_y Clip ])
           ]
       ]
     |> ok
     : unit View.t);
  let wide count = List.init count ~f:(fun _ -> View.text "item") in
  ignore (link (wide 4096) |> ok : unit View.t);
  assert (Or_error.is_error (link (wide 4097)));
  let deep depth =
    List.fold
      (List.init (depth - 1) ~f:Fn.id)
      ~init:(View.text "leaf")
      ~f:(fun child _ -> View.column [ child ])
  in
  ignore (link [ deep 128 ] |> ok : unit View.t);
  assert (Or_error.is_error (link [ deep 129 ]));
  print_endline
    "nested controls, links, selectable/scrolling/shielded descendants rejected; exact \
     depth/node bounds";
  [%expect
    {| nested controls, links, selectable/scrolling/shielded descendants rejected; exact depth/node bounds |}]
;;

let%expect_test "passive avatar and motion compose without child callbacks" =
  let config = Link.Config.create ~label:"Open workspace" () |> ok in
  let target = Animation.Target.create [ Width, 120. ] |> ok in
  let initial = Animation.Target.create [ Width, 60. ] |> ok in
  let tween = Animation.Config.create ~initial ~repeat:Alternate ~target () |> ok in
  let stage =
    Animation.Stage.create
      ~timing:(Animation.Timing.tween (Time_ns.Span.of_ms 200.) |> ok)
      ~target
      ()
    |> ok
  in
  let program = Animation.Program.create ~initial ~repeat:Loop [ stage ] |> ok in
  let avatar =
    Avatar.Config.create
      ~fallback:(Avatar.Fallback.create "DM" |> ok)
      ~description:Image.Description.decorative
      ()
  in
  let loading = Loading.Config.create ~kind:Spinner ~label:"Loading preview" () |> ok in
  let content =
    [ View.avatar avatar
    ; View.loading ~config:loading ()
    ; View.animate tween [ View.text "Tween" ]
    ; View.animate_program program [ View.text "Program" ]
    ]
  in
  let view = View.link config ~on_click:(fun () -> "root") content |> ok in
  let t = Reconciler.create window in
  let operations = commit t (Some view) in
  let handlers =
    List.filter_map operations ~f:(function
      | Wire.Op.Create (node, kind, _, Some handler) -> Some (node, kind, handler)
      | _ -> None)
  in
  let node, handler =
    match handlers with
    | [ (node, Wire.Kind.Link, handler) ] -> node, handler
    | _ -> failwith "passive content introduced a callback owner"
  in
  let event = Wire.Event.Press (window, node, handler, 1L) in
  assert (Option.equal String.equal (Reconciler.dispatch t event) (Some "root"));
  assert (List.is_empty (commit t (Some view)));
  let forbidden =
    [ View.animate ~on_event:(fun _ -> ()) tween [ View.text "Observed tween" ]
    ; View.animate_program
        ~on_event:(fun _ -> ())
        program
        [ View.text "Observed program" ]
    ]
  in
  List.iter forbidden ~f:(fun child ->
    assert (Or_error.is_error (View.link config ~on_click:(fun () -> ()) [ child ])));
  ignore (commit t None : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch t event));
  print_endline
    "avatar/loading/tween/program admitted; only root action; idle repeat; observed \
     descendants rejected; retired action fenced";
  [%expect
    {| avatar/loading/tween/program admitted; only root action; idle repeat; observed descendants rejected; retired action fenced |}]
;;

let%expect_test "styled link text accepts an outer highlight scope only" =
  let query = Highlight.Query.create "aaa" |> ok in
  let spec = Highlight.Spec.create ~query () |> ok in
  let highlight = Highlight.Config.create [ spec ] |> ok in
  let span =
    Text_content.Span.create
      ~start_byte:0
      ~end_byte:3
      ~foreground:(Color.rgb_exn 0x800080)
    |> ok
  in
  let text = Text_content.create ~spans:[ span ] "aaa 世界" |> ok in
  let config = Link.Config.create ~label:"Open result" () |> ok in
  let link children = View.link config ~on_click:(fun () -> ()) children in
  let content = View.styled_text text in
  let view =
    View.highlight_scope
      ~config:highlight
      ~style:(Style.create_exn [ User_select true ])
      [ link [ content ] |> ok ]
  in
  let t = Reconciler.create window in
  let operations = commit t (Some view) in
  assert (
    List.count operations ~f:(function
      | Wire.Op.Create (_, Link, _, Some _) -> true
      | _ -> false)
    = 1);
  assert (List.is_empty (commit t (Some view)));
  assert (Or_error.is_error (link [ View.highlight_scope ~config:highlight [ content ] ]));
  print_endline "styled content inside one link; scope outside; nested scope rejected";
  [%expect {| styled content inside one link; scope outside; nested scope rejected |}]
;;

let%expect_test "link fixture agrees with independent Rust bytes" =
  let config =
    Link.Config.create ~label:"Guide 世界" ~tab_stop:false ~tab_index:(-2) () |> ok
  in
  let wire = Link.Expert.to_wire config in
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire in
  let pos_ref = ref 0 in
  let decoded = W.bin_read_t bytes ~pos_ref in
  assert (!pos_ref = Bigstring.length bytes);
  assert (Link.Config.equal config (Link.Expert.of_wire decoded |> ok));
  Bigstring.to_string bytes
  |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0c477569646520e4b896e7958c0000fffe |}]
;;

let%expect_test "link configuration preserves explicit focus intent and defaults" =
  let defaults = Link.Config.create ~label:"Read the guide" () |> ok in
  assert (String.equal (Link.Config.label defaults) "Read the guide");
  assert (not (Link.Config.is_disabled defaults));
  assert (Link.Config.tab_stop defaults && Link.Config.tab_index defaults = 0);
  List.iter [ false; true ] ~f:(fun disabled ->
    List.iter [ false; true ] ~f:(fun tab_stop ->
      List.iter [ -1000000; -1; 0; 1000000 ] ~f:(fun tab_index ->
        let config =
          Link.Config.create ~label:"世界" ~disabled ~tab_stop ~tab_index () |> ok
        in
        assert (Bool.equal (Link.Config.is_disabled config) disabled);
        assert (Bool.equal (Link.Config.tab_stop config) tab_stop);
        assert (Link.Config.tab_index config = tab_index);
        assert (W.valid (Link.Expert.to_wire config)))));
  print_endline
    "enabled Tab default; signed indices and explicit tab policy survive disabled state";
  [%expect
    {| enabled Tab default; signed indices and explicit tab policy survive disabled state |}]
;;

let%expect_test "labels and wire values are validated at domain entry" =
  List.iter
    [ ""; " \t\n\r\011\012"; "a\000b"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Or_error.is_error (Link.Config.create ~label ())));
  ignore (Link.Config.create ~label:(String.make 4096 'x') () |> ok : Link.Config.t);
  let wire = Link.Config.create ~label:"Guide" () |> ok |> Link.Expert.to_wire in
  List.iter [ Int64.min_value; -1000001L; 1000001L; Int64.max_value ] ~f:(fun tab_index ->
    let invalid = { wire with tab_index } in
    assert (not (W.valid invalid));
    assert (Or_error.is_error (Link.Expert.of_wire invalid)));
  List.iter [ Int.min_value; -1000001; 1000001; Int.max_value ] ~f:(fun tab_index ->
    assert (Or_error.is_error (Link.Config.create ~label:"Guide" ~tab_index ())));
  assert (Or_error.is_error (Link.Expert.of_wire { wire with label = "\255" }));
  print_endline
    "invalid UTF-8/NUL/blank/oversize labels and out-of-range decoded indices rejected";
  [%expect
    {| invalid UTF-8/NUL/blank/oversize labels and out-of-range decoded indices rejected |}]
;;

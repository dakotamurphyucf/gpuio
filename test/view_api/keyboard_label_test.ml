open Core
open Gpuio
module K = Presentation.Kbd
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let p = Presentation.Appearance.dark
let chord ?(modifiers = []) key = Shortcut.create ~key ~modifiers () |> ok

let%expect_test "platform display follows physical modifier aliases and Unicode keys" =
  List.iter
    [ [ Shortcut.Modifier.Primary ], "a"
    ; [ Primary; Control; Alt; Shift; Super ], "a"
    ; [ Primary; Super ], "enter"
    ; [ Primary; Control ], "f24"
    ; [ Alt; Shift ], "backspace"
    ; [ Shift ], "delete"
    ; [ Control ], "+"
    ; [ Super ], "-"
    ; [], "é"
    ; [], "ß"
    ; [], "ﬃ"
    ; [], "😀"
    ]
    ~f:(fun (modifiers, key) ->
      let t = chord ~modifiers key in
      printf
        "%s | %s | %s\n"
        key
        (Shortcut.format t ~platform:Macos)
        (Shortcut.format t ~platform:Linux));
  [%expect
    {|
    a | ⌘A | Ctrl+A
    a | ⌃⌥⇧⌘A | Ctrl+Alt+Shift+Super+A
    enter | ⌘⏎ | Ctrl+Super+Enter
    f24 | ⌃⌘F24 | Ctrl+F24
    backspace | ⌥⇧⌫ | Alt+Shift+Backspace
    delete | ⇧⌦ | Shift+Delete
    + | ⌃+ | Ctrl++
    - | ⌘- | Super+-
    é | É | É
    ß | SS | SS
    ﬃ | FFI | FFI
    😀 | 😀 | 😀
    |}];
  List.iter
    [ "enter"
    ; "escape"
    ; "tab"
    ; "space"
    ; "backspace"
    ; "delete"
    ; "insert"
    ; "home"
    ; "end"
    ; "pageup"
    ; "pagedown"
    ; "left"
    ; "right"
    ; "up"
    ; "down"
    ; "f1"
    ; "f12"
    ]
    ~f:(fun key ->
      let t = chord key in
      printf
        "%s / %s\n"
        (Shortcut.format t ~platform:Macos)
        (Shortcut.format t ~platform:Linux));
  [%expect
    {|
    ⏎ / Enter
    ⎋ / Esc
    Tab / Tab
    Space / Space
    ⌫ / Backspace
    ⌦ / Delete
    Insert / Insert
    Home / Home
    End / End
    Page Up / Page Up
    Page Down / Page Down
    ← / Left
    → / Right
    ↑ / Up
    ↓ / Down
    F1 / F1
    F12 / F12
    |}]
;;

let%expect_test
    "spoken names and display ignore routing policy; declarations stay ordered"
  =
  let t = chord ~modifiers:[ Primary; Control; Alt; Shift; Super ] "left" in
  printf
    "%s\n%s\n"
    (Shortcut.accessible_label t ~platform:Macos)
    (Shortcut.accessible_label t ~platform:Linux);
  let routed =
    Shortcut.create
      ~key:"left"
      ~modifiers:[ Primary; Control; Alt; Shift; Super ]
      ~priority:Override
      ~text_input:Never
      ~during_composition:true
      ()
    |> ok
  in
  assert (
    String.equal
      (Shortcut.format t ~platform:Macos)
      (Shortcut.format routed ~platform:Macos));
  let make shortcuts =
    Command.create
      ~id:(Command.Id.of_string "save" |> ok)
      ~label:"Save"
      ~enabled:false
      ~shortcuts
      ~on_invoke:(fun () -> ())
      ()
    |> ok
  in
  assert (List.equal Shortcut.equal (Command.shortcuts (make [ routed; t ])) [ routed; t ]);
  assert (List.is_empty (Command.shortcuts (make [])));
  [%expect
    {|
    Control + Option + Shift + Command + Left arrow
    Control + Alt + Shift + Super + Left arrow
    |}]
;;

let fields view =
  Style.Expert.to_wire (View.Expert.describe view).style ~theme:Theme.default
  |> ok
  |> List.concat_map ~f:(function
    | W.Style.Fields f -> f
    | _ -> [])
;;

let%expect_test "keycaps validate names and preserve explicit style refinement" =
  let t = chord ~modifiers:[ Primary ] "k" in
  List.iter
    [ ""; "bad\000name"; "\255"; String.make 4097 'x' ]
    ~f:(fun accessible_name ->
      assert (Result.is_error (K.create p ~platform:Macos ~accessible_name t)));
  let filled = K.create p ~platform:Macos t |> ok in
  let outline = K.create p ~platform:Macos ~variant:Outline t |> ok in
  let plain = K.create p ~platform:Macos ~variant:Plain t |> ok in
  let contains view field = List.mem (fields view) field ~equal:W.Field.equal in
  assert (contains filled (Font_size 12.) && contains filled (Min_width (Px 20.)));
  assert (contains filled (Padding_left (Px 4.)) && contains filled (Padding_top (Px 2.)));
  assert (not (contains filled (Border_top_width 1.)));
  assert (contains outline (Border_top_width 1.));
  assert (List.is_empty (fields plain));
  List.iter [ K.Variant.Filled; Outline; Plain ] ~f:(fun variant ->
    let view =
      K.create
        p
        ~platform:Linux
        ~variant
        ~style:(Style.create_exn [ Font_size 18.; Padding (Length.px_exn 7.) ])
        ~accessible_name:"Commande localisée"
        t
      |> ok
    in
    let d = View.Expert.describe view in
    assert (String.equal d.text "Ctrl+K");
    assert (contains view (Font_size 18.) && contains view (Padding_top (Px 7.)));
    let metadata = Accessibility.Expert.to_wire (Option.value_exn d.accessibility) in
    assert (Option.equal String.equal metadata.label (Some "Commande localisée")));
  print_endline
    "filled/outline/plain defaults, all-variant refinements and localized name validation";
  [%expect
    {| filled/outline/plain defaults, all-variant refinements and localized name validation |}]
;;

let%expect_test "keycap updates retain one inert native text node" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let make platform variant =
    K.create p ~key:(Key.of_int 1) ~platform ~variant (chord ~modifiers:[ Primary ] "k")
    |> ok
  in
  let operations = commit (Some (make Macos Filled)) in
  let node =
    List.find_map_exn operations ~f:(function
      | W.Op.Create (id, Text, _, None) -> Some id
      | _ -> None)
  in
  List.iter [ K.Variant.Plain; Outline; Filled ] ~f:(fun variant ->
    List.iter [ Shortcut.Platform.Linux; Macos ] ~f:(fun platform ->
      let view = make platform variant in
      List.iter (commit (Some view)) ~f:(function
        | W.Op.Create _ | Remove _ -> failwith "keycap remounted"
        | _ -> ());
      assert (List.is_empty (commit (Some view)))));
  assert (
    List.exists (commit None) ~f:(function
      | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id node
      | _ -> false));
  print_endline
    "six platform/variant updates retain text identity; equal snapshots idle; unmount \
     removes node";
  [%expect
    {| six platform/variant updates retain text identity; equal snapshots idle; unmount removes node |}]
;;

let%expect_test "native keycaps preserve Function and raw key names without registration" =
  let module Binding = Command_binding in
  let config = Binding.Config.create [ Native_action Copy ] |> ok in
  let observation =
    Binding.Expert.of_wire
      config
      { Gpuio_protocol.Command_binding_wire.Observation.epoch = 1L
      ; state =
          Ready
            [ Native_binding
                { strokes = [ { key = "media-play"; modifiers = 24L } ]
                ; disposition = Widget
                }
            ]
      }
    |> Option.value_exn
  in
  let stroke =
    match Binding.Observation.state observation with
    | Ready [ (_, Native_binding { strokes = [ stroke ]; _ }) ] -> stroke
    | _ -> assert false
  in
  List.iter [ Shortcut.Platform.Macos; Linux ] ~f:(fun platform ->
    let view = K.of_native_stroke p ~platform ~variant:Plain stroke |> ok in
    let description = View.Expert.describe view in
    let metadata =
      Accessibility.Expert.to_wire (Option.value_exn description.accessibility)
    in
    assert (Option.is_none description.on_click && Option.is_none description.commands);
    printf "%s | %s\n" description.text (Option.value_exn metadata.label));
  assert (
    Result.is_error (K.of_native_stroke p ~platform:Macos ~accessible_name:"" stroke));
  [%expect
    {|
    fn⌘Media-play | Function + Command + Media-play
    Fn+Super+Media-play | Function + Super + Media-play
  |}]
;;

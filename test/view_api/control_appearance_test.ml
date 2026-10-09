open Core
open Gpuio
module A = Control_appearance
module W = Gpuio_protocol.Wire.Control_appearance

let ok = Or_error.ok_exn

let fixture name appearance =
  let wire = A.Expert.to_wire appearance ~theme:Theme.default |> ok in
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
    assert (String.equal hex expected))
;;

let%expect_test "independent control geometry and state fixtures match native bytes" =
  fixture "control-appearance-default.hex" A.default;
  let color = Color.rgba ~red:0x11 ~green:0x22 ~blue:0x33 ~alpha:0x44 |> ok in
  let appearance =
    A.create
      ~size:24.
      ~switch_width:48.
      ~gap:10.
      ~label_position:Before
      ~indicator_style:(Style.with_state_exn Style.empty Checked [ Foreground color ])
      ~mark_style:(Style.with_state_exn Style.empty Disabled [ Opacity 0.5 ])
      ()
    |> ok
  in
  fixture "control-appearance-states.hex" appearance;
  print_endline "independent defaults and before-label/checked/disabled fixtures";
  [%expect {| independent defaults and before-label/checked/disabled fixtures |}]
;;

let%expect_test "bounded geometry and part scopes reject unsupported presentation" =
  List.iter [ Float.nan; Float.infinity; -1.; 0.; 7.99; 128.01 ] ~f:(fun size ->
    assert (Result.is_error (A.create ~size ())));
  List.iter [ Float.nan; 17.99; 256.01 ] ~f:(fun switch_width ->
    assert (Result.is_error (A.create ~switch_width ())));
  List.iter [ Float.nan; -0.01; 128.01 ] ~f:(fun gap ->
    assert (Result.is_error (A.create ~gap ())));
  List.iter [ 8.; 128. ] ~f:(fun size ->
    ignore (A.create ~size ~switch_width:size ~gap:128. () |> ok : A.t));
  List.iter [ Style.State.Focused; Hovered; Pressed; Selected ] ~f:(fun state ->
    let part = Style.with_state_exn Style.empty state [ Opacity 0.5 ] in
    assert (Result.is_error (A.create ~indicator_style:part ()));
    assert (Result.is_error (A.create ~mark_style:part ())));
  List.iter
    [ Style.create_exn [ Width (Length.px_exn 20.) ]
    ; Style.create_exn [ Inert true ]
    ; Style.create_exn [ Font_size 14. ]
    ]
    ~f:(fun indicator_style -> assert (Result.is_error (A.create ~indicator_style ())));
  assert (Result.is_error (A.create ~mark_style:(Style.create_exn [ Radius 3. ]) ()));
  print_endline
    "finite geometry, endpoints, interaction states and structural fields checked";
  [%expect
    {| finite geometry, endpoints, interaction states and structural fields checked |}]
;;

let%expect_test "part theme resolution does not capture the initial theme" =
  let appearance =
    A.create
      ~indicator_style:(Style.create_exn [ Foreground (Color.token_exn "indicator") ])
      ()
    |> ok
  in
  assert (Result.is_error (A.Expert.to_wire appearance ~theme:Theme.default));
  let resolve color =
    A.Expert.to_wire
      appearance
      ~theme:(Theme.create [ "indicator", Color.rgb_exn color ] |> ok)
    |> ok
  in
  assert (not (W.equal (resolve 0x123456) (resolve 0xabcdef)));
  print_endline "unresolved token rejected; same appearance resolves under each theme";
  [%expect {| unresolved token rejected; same appearance resolves under each theme |}]
;;

let%expect_test
    "appearance operation and aggregate capability use paired independent bytes"
  =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let appearance = A.Expert.to_wire A.default ~theme:Theme.default |> ok in
  let hex bytes =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  let encoded =
    Wire.Message.encode
      (Apply
         { window
         ; base = 0L
         ; revision = 1L
         ; operations =
             [ Set_control_appearance (node, Some appearance)
             ; Set_control_appearance (node, None)
             ]
         })
    |> ok
    |> hex
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        encoded
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "control-appearance-operation.hex")
         |> String.strip)));
  assert (
    String.equal
      (Wire.Message.encode (Hello (Wire.version, 1152921504606846976L)) |> ok |> hex)
      "0003fc0000000000000010");
  assert (
    String.equal
      (Wire.Message.encode (Hello (Wire.version, Wire.capabilities)) |> ok |> hex)
      "0003fcffffffffffffff7f");
  print_endline "operation 67 set/reset; capability 60; independent bytes";
  [%expect {| operation 67 set/reset; capability 60; independent bytes |}]
;;

let%expect_test "appearance theme updates and reset keep all three control identities" =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let choice = Choice.Id.of_string "one" |> ok in
  let options =
    Choice.Collection.create [ Choice.create ~id:choice ~label:"One" () |> ok ] |> ok
  in
  let appearance =
    A.create
      ~indicator_style:(Style.create_exn [ Foreground (Color.token_exn "indicator") ])
      ()
    |> ok
  in
  let theme color =
    Theme.create
      [ "indicator", Color.rgb_exn color
      ; "foreground", Color.rgb_exn 0xcccccc
      ; "accent", Color.rgb_exn 0xffaa00
      ]
    |> ok
  in
  List.iter [ `Checkbox; `Switch; `Radio ] ~f:(fun kind ->
    let reconciler = Reconciler.create window in
    let view ?appearance ?(disabled = false) action =
      match kind with
      | `Checkbox ->
        View.checkbox
          ?appearance
          ~disabled
          ~state:Checked
          ~on_toggle:(fun () -> action)
          "Control"
      | `Switch ->
        View.switch
          ?appearance
          ~disabled
          ~checked:true
          ~on_toggle:(fun () -> action)
          "Control"
      | `Radio ->
        View.radio_group
          ?appearance
          ~config:
            (Choice.Config.create
               ~label:"Control"
               ~options
               ~selected:(Some choice)
               ~disabled
               ()
             |> ok)
          ~on_select:(fun _ -> action)
          ()
    in
    let commit theme view =
      let update = Reconciler.prepare reconciler ~theme (Some view) |> ok in
      Reconciler.accept reconciler update |> ok;
      match Reconciler.message update with
      | Some (Apply { operations; _ }) -> operations
      | None -> []
      | Some _ -> assert false
    in
    let first = commit (theme 0x112233) (view ~appearance 1) in
    let node, handler =
      List.find_map_exn first ~f:(function
        | Wire.Op.Create (node, _, _, Some handler) -> Some (node, handler)
        | _ -> None)
    in
    (match commit (theme 0xaabbcc) (view ~appearance 2) with
     | [ Set_control_appearance (same, Some value) ] ->
       assert (Gpuio_protocol.Node_id.equal node same);
       assert (W.equal value (A.Expert.to_wire appearance ~theme:(theme 0xaabbcc) |> ok))
     | _ -> assert false);
    assert (List.is_empty (commit (theme 0xaabbcc) (view ~appearance 3)));
    let event =
      match kind with
      | `Checkbox | `Switch -> Wire.Event.Press (window, node, handler, 1L)
      | `Radio -> Choice (window, node, handler, 1L, "one")
    in
    assert (Option.equal Int.equal (Reconciler.dispatch reconciler event) (Some 3));
    (match commit (theme 0xaabbcc) (view 4) with
     | [ Set_control_appearance (same, None) ] ->
       assert (Gpuio_protocol.Node_id.equal node same)
     | _ -> assert false);
    ignore (commit (theme 0xaabbcc) (view ~disabled:true 5) : Wire.Op.t list);
    assert (Option.is_none (Reconciler.dispatch reconciler event)));
  print_endline
    "checkbox/switch/radio: resolved updates, unchanged identity, current callbacks, \
     reset and disabled request rejection";
  [%expect
    {| checkbox/switch/radio: resolved updates, unchanged identity, current callbacks, reset and disabled request rejection |}]
;;

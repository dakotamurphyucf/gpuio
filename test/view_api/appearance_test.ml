open Core
open Gpuio
open Gpuio_protocol

let%expect_test "appearance validates geometry, text, state and structural overrides" =
  let invalid =
    [ Choice.Appearance.create ~row_height:0. ()
    ; Choice.Appearance.create ~popup_width:Float.nan ()
    ; Choice.Appearance.create ~max_visible_rows:65 ()
    ; Choice.Appearance.create ~empty_label:"\000" ()
    ; Choice.Appearance.create ~empty_label:"\255" ()
    ; Choice.Appearance.create
        ~popup_style:(Style.create_exn [ Width (Length.px_exn 20.) ])
        ()
    ; Choice.Appearance.create
        ~option_style:(Style.create_exn [ Pointer_events false ])
        ()
    ; Choice.Appearance.create
        ~empty_style:(Style.with_state_exn Style.empty Hovered [ Opacity 0.5 ])
        ()
    ]
  in
  assert (List.for_all invalid ~f:Result.is_error);
  assert (
    Result.is_ok
      (Choice.Appearance.create ~empty_label:"Keine Optionen" ~row_height:48. ()));
  print_endline "invalid geometry, text and unsupported appearance refinements rejected";
  [%expect {| invalid geometry, text and unsupported appearance refinements rejected |}]
;;

let%expect_test "theme changes update part colors without replacing the native choice" =
  let id value = Choice.Id.of_string value |> Or_error.ok_exn in
  let options =
    [ "fast"; "deep" ]
    |> List.map ~f:(fun key ->
      Choice.create ~id:(id key) ~label:key () |> Or_error.ok_exn)
    |> Choice.Collection.create
    |> Or_error.ok_exn
  in
  let config =
    Choice.Config.create ~label:"Mode" ~options ~selected:(Some (id "fast")) ()
    |> Or_error.ok_exn
  in
  let appearance =
    Choice.Appearance.create
      ~popup_style:(Style.create_exn [ Foreground (Color.token_exn "surface") ])
      ~option_style:
        (Style.with_state_exn
           Style.empty
           Focused
           [ Foreground (Color.token_exn "active") ])
      ~empty_label:"Keine Optionen"
      ()
    |> Or_error.ok_exn
  in
  let view = View.select ~config ~appearance ~on_select:Choice.Id.to_string () in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let reconciler = Reconciler.create window in
  let theme surface =
    Theme.create [ "surface", Color.rgb_exn surface; "active", Color.rgb_exn 0xabcdef ]
    |> Or_error.ok_exn
  in
  let commit theme =
    let update = Reconciler.prepare reconciler ~theme (Some view) |> Or_error.ok_exn in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = commit (theme 0x112233) in
  let node, handler =
    List.find_map_exn first ~f:(function
      | Create (node, Select, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let second = commit (theme 0x445566) in
  (match second with
   | [ Set_choice_appearance (same_node, appearance) ] ->
     assert (Node_id.equal node same_node);
     assert (String.equal appearance.empty_label "Keine Optionen");
     assert (
       List.equal
         Wire.Style.equal
         appearance.popup_style
         [ Fields [ Foreground (Rgba 0x445566ffL) ] ]);
     assert (
       List.equal
         Wire.Style.equal
         appearance.option_style
         [ State (1L, [ Foreground (Rgba 0xabcdefffL) ]) ])
   | _ -> assert false);
  print_s
    [%sexp
      (Reconciler.dispatch
         reconciler
         (Wire.Event.Choice (window, node, handler, 1L, "deep"))
       : string option)];
  assert (List.is_empty (commit (theme 0x445566)));
  print_endline
    "theme changes only appearance; stable callback and unchanged-theme no-op retained";
  [%expect
    {|
    (deep)
    theme changes only appearance; stable callback and unchanged-theme no-op retained
  |}]
;;

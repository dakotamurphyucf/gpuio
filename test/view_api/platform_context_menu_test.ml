open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let%expect_test "platform context presentation has independent paired bytes" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_menu
              ( node
              , { presentation = Platform_context
                ; menus =
                    [ { label = "Edit"; disabled = false; items = [ Command "copy" ] } ]
                } )
          ]
      }
  in
  let hex =
    W.Message.encode message
    |> ok
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "platform-context-menu-operation.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  print_endline "menu presentation 5; existing Set_menu operation";
  [%expect {| menu presentation 5; existing Set_menu operation |}]
;;

let%expect_test "platform icons validate paths and SVGs and preserve the target" =
  let owner = Asset.Expert.Owner.create () in
  let asset format =
    Asset.Expert.handle
      ~owner
      ~id:(Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok)
      ~format
  in
  let svg = asset Svg in
  let path = Menu.Item_path.of_list [ 0; 0 ] |> ok in
  let separator = Menu.Item_path.of_list [ 0; 1 ] |> ok in
  let missing = Menu.Item_path.of_list [ 0; 9 ] |> ok in
  let menu = Menu.create ~label:"Actions" [ Label "Section"; Separator ] |> ok in
  let original = View.context_menu ~platform:true ~menu (View.text "Target") in
  List.iter
    [ [ path, svg; path, svg ]
    ; [ separator, svg ]
    ; [ missing, svg ]
    ; [ path, asset Png ]
    ]
    ~f:(fun items -> assert (Result.is_error (View.with_menu_item_icons original ~items)));
  assert (
    Result.is_error
      (View.with_menu_item_icons
         (View.context_menu ~menu (View.text "Target"))
         ~items:[ path, svg ]));
  let decorated = View.with_menu_item_icons original ~items:[ path, svg ] |> ok in
  let description = View.Expert.describe decorated in
  assert (
    Menu.Expert.equal_presentation
      (Option.value_exn description.menu).presentation
      Platform_context);
  let children = List.map description.children ~f:View.Expert.describe in
  (match children with
   | [ target; slot; separator ] ->
     assert (String.equal target.text "Target");
     assert (List.is_empty separator.children);
     let icon = List.hd_exn slot.children |> View.Expert.describe in
     assert (View.Expert.Kind.equal icon.kind Icon);
     assert (List.is_empty icon.children)
   | _ -> assert false);
  let cleared =
    View.with_menu_item_icons decorated ~items:[] |> ok |> View.Expert.describe
  in
  assert (List.length cleared.children = 1);
  print_endline
    "SVG-only; paths validated; target retained; decoration-only slots; explicit clear";
  [%expect
    {| SVG-only; paths validated; target retained; decoration-only slots; explicit clear |}]
;;

let%expect_test "platform context is opt-in and rejects portable rich row content" =
  let menu = Menu.create ~label:"Actions" [ Label "Section" ] |> ok in
  let child = View.text "Target" in
  let native = View.context_menu ~platform:true ~menu child in
  let drawn = View.context_menu ~menu child in
  let presentation view =
    (View.Expert.describe view).menu
    |> Option.value_exn
    |> fun config -> config.View.Expert.presentation
  in
  assert (Menu.Expert.equal_presentation (presentation native) Platform_context);
  assert (Menu.Expert.equal_presentation (presentation drawn) Context);
  let path = Menu.Item_path.of_list [ 0; 0 ] |> ok in
  assert (
    Result.is_error (View.with_menu_item_content native ~items:[ path, View.text "Rich" ]));
  assert (
    Result.is_ok (View.with_menu_item_content drawn ~items:[ path, View.text "Rich" ]));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let prepared = Reconciler.prepare reconciler ~theme:Theme.default (Some native) |> ok in
  (match Reconciler.message prepared with
   | Some (W.Message.Apply { operations; _ }) ->
     assert (
       List.exists operations ~f:(function
         | W.Op.Set_menu (_, { presentation = Platform_context; _ }) -> true
         | _ -> false))
   | _ -> assert false);
  print_endline
    "opt-in platform presentation; default drawn; rich slots rejected; section label \
     admitted";
  [%expect
    {| opt-in platform presentation; default drawn; rich slots rejected; section label admitted |}]
;;

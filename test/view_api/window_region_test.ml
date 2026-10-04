open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "custom title bar reserves native controls and follows fullscreen" =
  List.iter [ Window.Backend.Macos; X11; Wayland ] ~f:(fun backend ->
    List.iter [ false; true ] ~f:(fun fullscreen ->
      let bar = View.title_bar ~backend ~fullscreen [] |> View.Expert.describe in
      assert (Option.equal Window_region.equal bar.window_region (Some Title_bar));
      let styles = Style.Expert.to_wire bar.style ~theme:Theme.default |> ok in
      let inset =
        List.find_map_exn styles ~f:(function
          | W.Style.Fields fields ->
            List.find_map fields ~f:(function
              | W.Field.Padding_left (Px value) -> Some value
              | _ -> None)
          | _ -> None)
      in
      print_s [%sexp (backend : Window.Backend.t), (fullscreen : bool), (inset : float)]));
  [%expect
    {|
    (Macos false 80)
    (Macos true 12)
    (X11 false 12)
    (X11 true 12)
    (Wayland false 12)
    (Wayland true 12) |}]
;;

let%expect_test "window regions preserve nodes and clear through reconciliation" =
  let r = Reconciler.create window in
  let view region =
    View.column [ View.button ~on_click:(fun () -> ()) "Child" ]
    |> fun view -> View.with_window_region view region |> ok
  in
  let commit region =
    let update = Reconciler.prepare r ~theme:Theme.default (Some (view region)) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = commit (Some Title_bar) in
  let node =
    List.find_map_exn first ~f:(function
      | W.Op.Set_window_region (node, Some Title_bar) -> Some node
      | _ -> None)
  in
  assert (List.is_empty (commit (Some Title_bar)));
  List.iter
    [ Some Window_region.Exclude; Some (Resize Bottom_right); None ]
    ~f:(fun region ->
      let operations = commit region in
      assert (List.length operations = 1);
      match operations with
      | [ W.Op.Set_window_region (id, actual) ] ->
        assert (Gpuio_protocol.Node_id.equal id node);
        assert (
          Option.equal
            Gpuio_protocol.Window_region_wire.equal
            actual
            (Option.map region ~f:Window_region.Expert.to_wire))
      | _ -> assert false);
  print_endline "region updates and removal retain child identity";
  [%expect {| region updates and removal retain child identity |}]
;;

let%expect_test "regions belong to containers and cannot enter passive slots" =
  assert (Result.is_error (View.with_window_region (View.text "Text") (Some Title_bar)));
  let content =
    View.column [] |> fun v -> View.with_window_region v (Some Title_bar) |> ok
  in
  assert (
    Result.is_error
      (View.button_with_content
         ~accessible_name:"Unsafe"
         ~on_click:(fun () -> ())
         content));
  let passive = View.with_window_region content None |> ok in
  assert (
    Result.is_ok
      (View.button_with_content ~accessible_name:"Safe" ~on_click:(fun () -> ()) passive));
  [%expect {| |}]
;;

let%expect_test "all window regions have independent epoch-three wire fixtures" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  List.iter
    [ None
    ; Some Gpuio_protocol.Window_region_wire.Title_bar
    ; Some Exclude
    ; Some (Resize Top)
    ; Some (Resize Bottom)
    ; Some (Resize Left)
    ; Some (Resize Right)
    ; Some (Resize Top_left)
    ; Some (Resize Top_right)
    ; Some (Resize Bottom_left)
    ; Some (Resize Bottom_right)
    ]
    ~f:(fun region ->
      let bytes =
        W.Message.encode
          (Apply
             { window
             ; base = 0L
             ; revision = 1L
             ; operations = [ Set_window_region (node, region) ]
             })
        |> ok
      in
      print_endline
        (String.to_list bytes
         |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
         |> String.concat));
  [%expect
    {|
    0300010001017a000100
    0300010001017a00010100
    0300010001017a00010101
    0300010001017a0001010200
    0300010001017a0001010201
    0300010001017a0001010202
    0300010001017a0001010203
    0300010001017a0001010204
    0300010001017a0001010205
    0300010001017a0001010206
    0300010001017a0001010207 |}]
;;

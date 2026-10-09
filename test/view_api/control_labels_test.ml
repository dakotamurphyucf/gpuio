open Core
open Gpuio
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let name = "Accessible control"

let rich kind ?(accessible_name = name) label action =
  match kind with
  | `Checkbox ->
    View.checkbox_with_label
      ~key:(key "owner")
      ~accessible_name
      ~state:Checked
      ~on_toggle:(fun () -> action)
      label
  | `Switch ->
    View.switch_with_label
      ~key:(key "owner")
      ~accessible_name
      ~checked:true
      ~on_toggle:(fun () -> action)
      label
;;

let plain kind action =
  match kind with
  | `Checkbox ->
    View.checkbox
      ~key:(key "owner")
      ~accessible_name:name
      ~state:Checked
      ~on_toggle:(fun () -> action)
      name
  | `Switch ->
    View.switch
      ~key:(key "owner")
      ~accessible_name:name
      ~checked:true
      ~on_toggle:(fun () -> action)
      name
;;

let commit reconciler view =
  let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept reconciler update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let window () = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node_equal = Gpuio_protocol.Node_id.equal

let%expect_test "rich checkable labels validate names and passive contents" =
  List.iter [ `Checkbox; `Switch ] ~f:(fun kind ->
    List.iter
      [ ""; "  "; "\t\n\011\012\r"; "\000"; "\255"; String.make 1025 'x' ]
      ~f:(fun accessible_name ->
        assert (Result.is_error (rich kind ~accessible_name (View.text "Visible") ())));
    ignore
      (rich kind ~accessible_name:(String.make 1024 'x') (View.text "") () |> ok
       : unit View.t);
    ignore
      (rich kind ~accessible_name:"\194\160" (View.text "Caption") () |> ok : unit View.t);
    List.iter
      [ View.button ~on_click:(fun () -> ()) "Nested action"
      ; View.text ~style:(Style.create_exn [ User_select true ]) "Selection"
      ; View.column
          ~style:(Style.with_state_exn Style.empty Hovered [ Overflow_y Scroll ])
          []
      ]
      ~f:(fun label -> assert (Result.is_error (rich kind label ())));
    let nested =
      List.fold (List.init 129 ~f:Fn.id) ~init:(View.text "Deep") ~f:(fun child _ ->
        View.column [ child ])
    in
    assert (Result.is_error (rich kind nested ()));
    assert (
      Result.is_error
        (rich kind (View.column (List.init 4096 ~f:(fun _ -> View.text "Many"))) ())));
  print_endline
    "checked names, nested controls, state-layer input policy and bounded label trees";
  [%expect
    {| checked names, nested controls, state-layer input policy and bounded label trees |}]
;;

let%expect_test "plain and rich descriptions keep the control and current callback" =
  List.iter [ `Checkbox; `Switch ] ~f:(fun kind ->
    let window = window () in
    let r = Reconciler.create window in
    let initial = commit r (plain kind 1) in
    let owner, handler =
      List.find_map_exn initial ~f:(function
        | Wire.Op.Create (node, _, _, Some handler) -> Some (node, handler)
        | _ -> None)
    in
    let label text = View.text ~key:(key "label") text in
    let operations = commit r (rich kind (label "Rich label") 2 |> ok) in
    let child =
      List.find_map_exn operations ~f:(function
        | Wire.Op.Create (node, Text, "Rich label", None) -> Some node
        | _ -> None)
    in
    assert (
      not
        (List.exists operations ~f:(function
           | Wire.Op.Remove node -> node_equal owner node
           | _ -> false)));
    let operations = commit r (rich kind (label "Updated label") 3 |> ok) in
    assert (
      List.exists operations ~f:(function
        | Wire.Op.Set_text (node, "Updated label") -> node_equal child node
        | _ -> false));
    assert (
      not
        (List.exists operations ~f:(function
           | Wire.Op.Create _ | Remove _ -> true
           | _ -> false)));
    let event = Wire.Event.Press (window, owner, handler, 1L) in
    assert (Option.equal Int.equal (Reconciler.dispatch r event) (Some 3));
    let operations = commit r (plain kind 4) in
    assert (
      List.exists operations ~f:(function
        | Wire.Op.Remove node -> node_equal child node
        | _ -> false));
    assert (
      not
        (List.exists operations ~f:(function
           | Wire.Op.Remove node -> node_equal owner node
           | _ -> false)));
    assert (Option.equal Int.equal (Reconciler.dispatch r event) (Some 4)));
  print_endline
    "checkbox/switch owners retained; keyed label updated then retired; latest callbacks";
  [%expect
    {| checkbox/switch owners retained; keyed label updated then retired; latest callbacks |}]
;;

let%expect_test "radio/tab labels use option identity with partial fallback and reorder" =
  List.iter [ false; true ] ~f:(fun tabs ->
    let id = fun text -> Choice.Id.of_string text |> ok in
    let config ids =
      Choice.Config.create
        ~label:"Mode"
        ~options:
          (Choice.Collection.create
             (List.map ids ~f:(fun value ->
                Choice.create ~id:(id value) ~label:("Plain " ^ value) () |> ok))
           |> ok)
        ~selected:(Some (id "a"))
        ()
      |> ok
    in
    let labels = [ id "a", View.text ~key:(key "caption") "Rich A" ] in
    let make ids labels action =
      if tabs
      then
        View.tab_bar_with_labels
          ~key:(key "group")
          ~config:(config ids)
          ~labels
          ~on_select:(fun _ -> action)
          ()
      else
        View.radio_group_with_labels
          ~key:(key "group")
          ~config:(config ids)
          ~labels
          ~on_select:(fun _ -> action)
          ()
    in
    assert (Result.is_error (make [ "a"; "b" ] (labels @ labels) 1));
    assert (
      Result.is_error
        (make
           [ "a"; "b" ]
           [ id "a", View.button ~on_click:(fun () -> 1) "Nested action" ]
           1));
    assert (Result.is_error (make [ "a"; "b" ] [ id "unknown", View.text "Unknown" ] 1));
    let first = make [ "a"; "b" ] labels 1 |> ok in
    let slots = (View.Expert.describe first).children in
    assert (
      List.equal
        Int.equal
        (List.map slots ~f:(fun slot -> List.length (View.Expert.describe slot).children))
        [ 1; 0 ]);
    let window = window () in
    let r = Reconciler.create window in
    let initial = commit r first in
    let owner, handler =
      List.find_map_exn initial ~f:(function
        | Wire.Op.Create (node, kind, _, Some handler)
          when Wire.Kind.equal kind (if tabs then Tab_bar else Radio_group) ->
          Some (node, handler)
        | _ -> None)
    in
    let operations = commit r (make [ "b"; "a" ] labels 2 |> ok) in
    assert (
      not
        (List.exists operations ~f:(function
           | Wire.Op.Create _ | Remove _ -> true
           | _ -> false)));
    assert (
      List.exists operations ~f:(function
        | Wire.Op.Splice (node, _, _, _) -> node_equal node owner
        | _ -> false));
    assert (
      Option.equal
        Int.equal
        (Reconciler.dispatch r (Wire.Event.Choice (window, owner, handler, 1L, "a")))
        (Some 2));
    let operations = commit r (make [ "b"; "a" ] [] 3 |> ok) in
    assert (
      List.exists operations ~f:(function
        | Wire.Op.Remove _ -> true
        | _ -> false));
    assert (
      not
        (List.exists operations ~f:(function
           | Wire.Op.Remove node -> node_equal node owner
           | _ -> false))));
  print_endline
    "unique known IDs, string fallback, keyed reorder without remount, slot retirement";
  [%expect
    {| unique known IDs, string fallback, keyed reorder without remount, slot retirement |}]
;;

let%expect_test "rich label capability has independent bytes" =
  let hex message =
    Wire.Message.encode message
    |> ok
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        (hex (Hello (Wire.version, 2305843009213693952L)))
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "control-labels-hello.hex")
         |> String.strip)));
  assert (
    String.equal (hex (Hello (Wire.version, Wire.capabilities))) "0003fcffffffffffffff7f");
  print_endline "capability 61 and full paired mask";
  [%expect {| capability 61 and full paired mask |}]
;;

open Core
open Gpuio
module W = Gpuio_protocol.List_input_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let fixture name =
  Eio_main.run (fun env ->
    let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
    String.init
      (String.length hex / 2)
      ~f:(fun i ->
        Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2))))
;;

let node slot = Gpuio_protocol.Node_id.create ~slot ~generation:1L |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok
let dump writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let requests =
  W.Request.
    [ Navigate (Next, Some (Range { extend = true }))
    ; Select (42L, Toggle)
    ; Focus 42L
    ; Select_active Replace
    ; Confirm (42L, Secondary)
    ; Confirm_active Primary
    ; Context 42L
    ; Context_active
    ; Set_selected (42L, false)
    ; Cancel
    ; Navigate (First, None)
    ; Confirm_active Secondary
    ]
;;

let config =
  W.Config.
    { generation = 7L
    ; cursor = Some 42L
    ; query = Some (node 2L)
    ; selection_on_navigation = true
    ; disabled = false
    ; busy = true
    }
;;

let event request = Wire.Event.List_input (window, node 0L, handler, 1L, 7L, request)

let%expect_test "independent paired list input preserves ordered intents and old tags" =
  assert (
    String.equal
      (dump [%bin_writer: W.Request.t list] requests)
      (fixture "list-input-requests.hex"));
  assert (List.for_all requests ~f:W.Request.valid);
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_list_input (node 0L, Some config); Set_list_input (node 0L, None) ]
      }
  in
  assert (
    String.equal
      (Wire.Message.encode message |> ok)
      (fixture "list-input-transaction.hex"));
  let value = event (List.hd_exn requests) in
  let bytes = fixture "list-input-event.hex" in
  assert (String.equal (dump Wire.Event.bin_writer_t value) bytes);
  assert (List.equal Wire.Event.equal [ value ] (Wire.Event.decode ("\001" ^ bytes) |> ok));
  let events = List.map requests ~f:event in
  assert (
    List.equal
      Wire.Event.equal
      events
      (Wire.Event.decode (dump [%bin_writer: Wire.Event.t list] events) |> ok));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode ("\001" ^ String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode ("\001" ^ bytes ^ "\000")));
  List.iter [ 0L; -1L ] ~f:(fun id ->
    List.iter
      W.Request.
        [ Select (id, Replace)
        ; Focus id
        ; Confirm (id, Primary)
        ; Context id
        ; Set_selected (id, true)
        ]
      ~f:(fun request ->
        assert (not (W.Request.valid request));
        assert (
          Result.is_error
            (Wire.Event.decode (dump [%bin_writer: Wire.Event.t list] [ event request ])))));
  List.iter [ -1L; 0L ] ~f:(fun generation ->
    let value =
      Wire.Event.List_input (window, node 0L, handler, 1L, generation, Cancel)
    in
    assert (
      Result.is_error
        (Wire.Event.decode (dump [%bin_writer: Wire.Event.t list] [ value ]))));
  let invalid_revision =
    Wire.Event.List_input (window, node 0L, handler, -1L, 7L, Cancel)
  in
  assert (
    Result.is_error
      (Wire.Event.decode (dump [%bin_writer: Wire.Event.t list] [ invalid_revision ])));
  (* Unknown request, navigation and gesture tags reject at the generated reader. *)
  List.iter
    [ 9, 10; 10, 4; 12, 3 ]
    ~f:(fun (offset, tag) ->
      let invalid =
        String.mapi bytes ~f:(fun index char ->
          if index = offset then Char.of_int_exn tag else char)
      in
      assert (Result.is_error (Wire.Event.decode ("\001" ^ invalid))));
  assert (not (W.Config.valid { config with generation = 0L }));
  assert (not (W.Config.valid { config with cursor = Some 0L }));
  [%expect {| |}]
;;

let%expect_test "checked option metadata matches the wire and follows row wrappers" =
  let list = Accessibility.create ~role:(List_box true) ~label:"Pick" () |> ok in
  let item = Accessibility.Option_item.create ~index:4 ~count:9 ~selected:true () |> ok in
  let option = Accessibility.create ~role:(Option_item item) ~label:"Item" () |> ok in
  let dump metadata =
    dump
      Gpuio_protocol.Accessibility_wire.Config.bin_writer_t
      (Accessibility.Expert.to_wire metadata)
  in
  assert (String.equal (dump list) (fixture "accessibility-listbox.hex"));
  assert (String.equal (dump option) (fixture "accessibility-option.hex"));
  List.iter
    [ -1, None; 1_000_000, None; 4, Some 4; 4, Some 1_000_001 ]
    ~f:(fun (index, count) ->
      assert (Result.is_error (Accessibility.Option_item.create ~index ?count ())));
  assert (Result.is_ok (Accessibility.Option_item.create ~index:999_999 ()));
  let key = Key.of_string_exn "one" in
  let row =
    View.column [ View.text "Item" ]
    |> fun view -> View.with_accessibility view option |> ok
  in
  let config = Virtual_list.Config.create ~height:(Fixed 24.) () |> ok in
  let view = View.virtual_list ~config [ key, row ] |> ok in
  let view = View.with_accessibility view list |> ok in
  let root = View.Expert.describe view in
  assert (Option.equal Accessibility.equal root.accessibility (Some list));
  let row = List.hd_exn root.children |> View.Expert.describe in
  assert (Option.equal Accessibility.equal row.accessibility (Some option));
  assert (Option.is_none (List.hd_exn row.children |> View.Expert.describe).accessibility);
  assert (Result.is_error (View.with_accessibility (View.column []) list));
  [%expect {| |}]
;;

let public_view
      ?(attached = true)
      ?(query = true)
      ?(cursor = "a")
      ?(busy = false)
      ?(epoch = "source")
      ()
  =
  let key = Key.of_string_exn in
  let rows =
    List.mapi [ "a"; "b" ] ~f:(fun index name ->
      let item =
        Accessibility.Option_item.create ~index ~count:2 ~selected:(index = 0) () |> ok
      in
      let metadata = Accessibility.create ~role:(Option_item item) ~label:name () |> ok in
      key name, View.with_accessibility (View.column [ View.text name ]) metadata |> ok)
  in
  let list =
    View.virtual_list
      ~key:(key "results")
      ~config:(Virtual_list.Config.create ~height:(Fixed 24.) () |> ok)
      ~on_viewport:(fun _ -> ())
      rows
    |> ok
  in
  let list =
    View.with_accessibility
      list
      (Accessibility.create ~role:(List_box true) ~label:"Results" () |> ok)
    |> ok
  in
  let list =
    if not attached
    then list
    else
      View.with_list_input
        list
        ~config:
          (List_input.Config.create
             ~epoch:(key epoch)
             ~cursor:(key cursor)
             ?query:(if query then Some (key "search") else None)
             ~busy
             ())
        ~on_input:(fun _ -> ())
      |> ok
  in
  let children =
    if not query
    then [ list ]
    else
      [ list
      ; View.text_input
          ~controller:(key "search")
          ~config:(Text_input.Config.create ~mode:Single_line ~label:"Search" () |> ok)
          ~on_event:(fun _ -> ())
          ()
        |> ok
      ]
  in
  View.column children
;;

let%expect_test "public list input transactions replay in the native owner" =
  let reconciler = Reconciler.create window in
  let views =
    [ Some (public_view ())
    ; Some (public_view ~cursor:"b" ~busy:true ())
    ; Some (public_view ~epoch:"query2" ())
    ; Some (public_view ~attached:false ~query:false ())
    ; Some (public_view ~query:false ())
    ; None
    ]
  in
  List.iteri views ~f:(fun index view ->
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    let message = Option.value_exn (Reconciler.message update) in
    let bytes = Wire.Message.encode message |> ok in
    assert (String.equal bytes (fixture (sprintf "list-input-public-%d.hex" index)));
    Reconciler.accept reconciler update |> ok);
  Reconciler.close reconciler;
  [%expect {| |}]
;;

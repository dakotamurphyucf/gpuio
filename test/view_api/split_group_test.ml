open Core
open Gpuio
module W = Gpuio_protocol.Split_group_wire

let ok = Or_error.ok_exn
let id s = Split_group.Id.of_string s |> ok

let panels () =
  [ Split_group.Panel.create
      (id "a")
      ~label:"Files"
      ~initial_size:120.
      ~minimum_size:80.
      ~maximum_size:400.
      ()
    |> ok
  ; Split_group.Panel.create
      (id "b")
      ~label:"Editor"
      ~minimum_size:40.
      ~maximum_size:600.
      ()
    |> ok
  ; Split_group.Panel.create
      (id "c")
      ~label:"Inspector"
      ~initial_size:90.
      ~minimum_size:20.
      ~maximum_size:300.
      ~visible:false
      ()
    |> ok
  ]
;;

let config () =
  Split_group.Config.create
    ~label:"Workspace"
    ~axis:Vertical
    ~keyboard_step:8.
    ~reset_generation:2L
    ~resize:(Split_group.Resize_request.create (id "b") ~size:175. ~serial:3L |> ok)
    (panels ())
  |> ok
;;

let fixture name =
  Eio_main.run (fun env ->
    Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip)
;;

let hex writer value =
  Bin_prot.Utils.bin_dump writer value
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "handle appearance checks paint-only styles and independent bytes" =
  List.iter [ Float.nan; Float.infinity; 0.; 16.1 ] ~f:(fun thickness ->
    assert (Result.is_error (Split_group.Appearance.create ~thickness ())));
  List.iter [ Float.nan; 7.9; 32.1 ] ~f:(fun hit_extent ->
    assert (Result.is_error (Split_group.Appearance.create ~hit_extent ())));
  assert (Result.is_error (Split_group.Appearance.create ~thickness:10. ~hit_extent:8. ()));
  List.iter
    [ Style.create_exn [ Width (Length.px_exn 20.) ]
    ; Style.create_exn [ Display Hidden ]
    ; Style.create_exn [ Overflow_x Scroll ]
    ; Style.create_exn [ User_select true ]
    ; Style.with_state_exn Style.empty Selected [ Opacity 0.5 ]
    ; Style.with_state_exn Style.empty Checked [ Opacity 0.5 ]
    ]
    ~f:(fun handle_style ->
      assert (Result.is_error (Split_group.Appearance.create ~handle_style ())));
  assert (
    Result.is_error
      (Split_group.Appearance.create
         ~item_styles:[ id "a", Style.empty; id "a", Style.empty ]
         ()));
  assert (
    Result.is_error
      (Split_group.Appearance.create
         ~item_styles:(List.init 65 ~f:(fun i -> id (Int.to_string i), Style.empty))
         ()));
  let many_states =
    List.fold
      [ Style.State.Hovered; Focused; Pressed; Disabled ]
      ~init:(Style.create_exn [ Opacity 0.9 ])
      ~f:(fun style state -> Style.with_state_exn style state [ Opacity 0.5 ])
  in
  assert (
    Result.is_error
      (Split_group.Appearance.create
         ~item_styles:(List.init 64 ~f:(fun i -> id (Int.to_string i), many_states))
         ()));
  let appearance =
    Split_group.Appearance.create
      ~thickness:3.
      ~hit_extent:16.
      ~handle_style:(Style.create_exn [ Opacity 0.75 ])
      ~item_styles:[ id "a", Style.with_state_exn Style.empty Hovered [ Opacity 0.5 ] ]
      ()
    |> ok
    |> fun t -> Split_group.Expert.appearance_to_wire t ~theme:Theme.default |> ok
  in
  assert (
    String.equal
      (hex Gpuio_protocol.Wire.Split_group_appearance.bin_writer_t appearance)
      (fixture "split-group-appearance.hex"));
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  assert (
    String.equal
      (hex
         Gpuio_protocol.Wire.Op.bin_writer_t
         (Set_split_group (node, Split_group.Expert.to_wire (config ()), appearance)))
      (fixture "split-group-operation.hex"));
  let event_hex = fixture "split-group-event.hex" in
  let event_bytes =
    String.init
      (String.length event_hex / 2)
      ~f:(fun i ->
        Char.of_int_exn (Int.of_string ("0x" ^ String.sub event_hex ~pos:(i * 2) ~len:2)))
  in
  (match Gpuio_protocol.Wire.Event.decode event_bytes |> ok with
   | [ Split_group_resized (_, _, _, revision, generation, snapshot) ] ->
     assert (Int64.equal revision 4L && Int64.equal generation 2L);
     ignore
       (Split_group.Expert.snapshot_of_wire (config ()) snapshot |> ok
        : Split_group.Snapshot.t)
   | _ -> assert false);
  print_endline
    "bounded hit geometry, paint-only states, IDs and total style budget; independent \
     bytes";
  [%expect
    {| bounded hit geometry, paint-only states, IDs and total style budget; independent bytes |}]
;;

let%expect_test "split group public values match independent config and snapshot bytes" =
  let config = config () in
  assert (
    String.equal
      (hex W.Config.bin_writer_t (Split_group.Expert.to_wire config))
      (fixture "split-group-config.hex"));
  let snapshot =
    { W.Snapshot.source = Request 3L; sizes = [ "a", 120.; "b", 175.; "c", 90. ] }
  in
  assert (
    String.equal
      (hex W.Snapshot.bin_writer_t snapshot)
      (fixture "split-group-snapshot.hex"));
  let public = Split_group.Expert.snapshot_of_wire config snapshot |> ok in
  print_s
    [%sexp
      (List.map (Split_group.Snapshot.sizes public) ~f:(fun (id, size) ->
         Split_group.Id.to_string id, size)
       : (string * float) list)];
  print_s [%sexp (Split_group.Snapshot.source public : Split_group.Source.t)];
  [%expect
    {| ((a 120) (b 175) (c 90))
 (Request 3) |}]
;;

let%expect_test "checked IDs ranges requests and bounded unique collections" =
  List.iter
    [ ""; "bad\000id"; String.make 257 'x'; "\255" ]
    ~f:(fun s -> assert (Result.is_error (Split_group.Id.of_string s)));
  List.iter [ Float.nan; Float.infinity; -1.; 16385. ] ~f:(fun size ->
    assert (
      Result.is_error (Split_group.Panel.create (id "a") ~label:"A" ~initial_size:size ()));
    assert (Result.is_error (Split_group.Resize_request.create (id "a") ~size ~serial:1L)));
  List.iter [ 0L; -1L ] ~f:(fun serial ->
    assert (
      Result.is_error (Split_group.Resize_request.create (id "a") ~size:100. ~serial)));
  assert (Result.is_error (Split_group.Panel.create (id "a") ~label:" " ()));
  assert (
    Result.is_error
      (Split_group.Panel.create
         (id "a")
         ~label:"A"
         ~minimum_size:100.
         ~maximum_size:50.
         ()));
  assert (
    Result.is_error (Split_group.Panel.create (id "a") ~label:"A" ~initial_size:40. ()));
  let a = List.hd_exn (panels ()) in
  assert (Result.is_error (Split_group.Config.create ~label:"group" [ a; a ]));
  assert (
    Result.is_error (Split_group.Config.create ~label:"group" ~reset_generation:(-1L) []));
  assert (Result.is_error (Split_group.Config.create ~label:"group" ~keyboard_step:0. []));
  let many =
    List.init 65 ~f:(fun i ->
      Split_group.Panel.create (id (Int.to_string i)) ~label:"Panel" () |> ok)
  in
  assert (Result.is_error (Split_group.Config.create ~label:"group" many));
  ignore
    (Split_group.Config.create ~label:"group" (List.take many 64) |> ok
     : Split_group.Config.t);
  ignore (Split_group.Config.create ~label:"empty" [] |> ok : Split_group.Config.t);
  ignore
    (Split_group.Config.create
       ~label:"missing request target"
       ~resize:
         (Split_group.Resize_request.create (id "missing") ~size:100. ~serial:1L |> ok)
       []
     |> ok
     : Split_group.Config.t);
  print_endline
    "finite bounds, unique IDs, positive request serials, 64 panels; empty and missing \
     targets valid";
  [%expect
    {| finite bounds, unique IDs, positive request serials, 64 panels; empty and missing targets valid |}]
;;

let%expect_test "native snapshots are tied to current IDs order and complete ranges" =
  let config = config () in
  List.iter
    [ [ "b", 120.; "a", 175.; "c", 90. ]
    ; [ "a", 79.; "b", 175.; "c", 90. ]
    ; [ "a", 120.; "b", 601.; "c", 90. ]
    ; [ "a", 120.; "b", 175. ]
    ; [ "a", 120.; "a", 175.; "c", 90. ]
    ]
    ~f:(fun sizes ->
      assert (
        Result.is_error
          (Split_group.Expert.snapshot_of_wire
             config
             { W.Snapshot.source = Pointer; sizes })));
  assert (
    Result.is_error
      (Split_group.Expert.snapshot_of_wire
         config
         { W.Snapshot.source = Request 0L; sizes = [ "a", 120.; "b", 175.; "c", 90. ] }));
  print_endline "reorder, missing/duplicate IDs, bounds and invalid sources rejected";
  [%expect {| reorder, missing/duplicate IDs, bounds and invalid sources rejected |}]
;;

let%expect_test "mounted split groups preserve keyed children and fence resize history" =
  let module P = Gpuio_protocol in
  let module Wire = P.Wire in
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let config = config () in
  let panels =
    [ id "a", View.button ~on_click:(fun () -> 9) "Action"
    ; id "b", View.text "Editor"
    ; id "c", View.text "Inspector"
    ]
  in
  let make ?appearance config action =
    View.split_group
      ~config
      ~panels
      ?appearance
      ~handles:[ id "a", View.text "Grip" ]
      ~on_resize:(fun _ -> action)
      ()
    |> ok
  in
  let commit view =
    let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = commit (Some (make config 1)) in
  let node, handler =
    List.find_map_exn first ~f:(function
      | Wire.Op.Create (n, Split_group, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let snapshot =
    { W.Snapshot.source = Pointer; sizes = [ "a", 120.; "b", 175.; "c", 90. ] }
  in
  let event = Wire.Event.Split_group_resized (window, node, handler, 1L, 2L, snapshot) in
  assert (Option.equal Int.equal (Reconciler.dispatch r event) (Some 1));
  let appearance = Split_group.Appearance.create ~thickness:3. () |> ok in
  let paint = commit (Some (make ~appearance config 2)) in
  assert (
    not
      (List.exists paint ~f:(function
         | Wire.Op.Bind _ | Create _ | Remove _ -> true
         | _ -> false)));
  assert (Option.equal Int.equal (Reconciler.dispatch r event) (Some 2));
  assert (List.is_empty (commit (Some (make ~appearance config 2))));
  let reordered =
    Split_group.Config.create
      ~label:"Workspace"
      ~reset_generation:2L
      (List.rev (Split_group.Config.panels config))
    |> ok
  in
  let changed = commit (Some (make ~appearance reordered 3)) in
  assert (
    not
      (List.exists changed ~f:(function
         | Wire.Op.Create _ | Remove _ -> true
         | _ -> false)));
  assert (
    List.exists changed ~f:(function
      | Wire.Op.Bind (n, Some _) -> P.Node_id.equal node n
      | _ -> false));
  assert (Option.is_none (Reconciler.dispatch r event));
  let older =
    Split_group.Config.create ~label:"Workspace" (Split_group.Config.panels config) |> ok
  in
  assert (
    Result.is_error (Reconciler.prepare r ~theme:Theme.default (Some (make older 4))));
  ignore (commit None : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch r event));
  assert (Result.is_error (View.split_group ~config ~panels:[] ()));
  assert (
    Result.is_error
      (View.split_group ~config ~panels:(panels @ [ id "a", View.text "dup" ]) ()));
  assert (
    Result.is_error
      (View.split_group ~config ~panels ~handles:[ id "missing", View.text "x" ] ()));
  assert (
    Result.is_error
      (View.split_group
         ~config
         ~panels
         ~handles:[ id "a", View.button ~on_click:(fun () -> 1) "bad" ]
         ()));
  print_endline
    "keyed content survives reorder; appearance retains handler; config and disposal \
     fence events; slots and passive grips checked";
  [%expect
    {| keyed content survives reorder; appearance retains handler; config and disposal fence events; slots and passive grips checked |}]
;;

let%expect_test "public split group transactions replay in the native tree" =
  let module P = Gpuio_protocol in
  let module Wire = P.Wire in
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let view names ~hidden ~generation ~resize =
    let config =
      Split_group.Config.create
        ~label:"Public workspace"
        ~reset_generation:generation
        ?resize
        (List.map names ~f:(fun name ->
           Split_group.Panel.create
             (id name)
             ~label:name
             ~initial_size:100.
             ~minimum_size:40.
             ~maximum_size:500.
             ~visible:(not (Option.exists hidden ~f:(String.equal name)))
             ()
           |> ok))
      |> ok
    in
    View.split_group
      ~key:(Key.of_string_exn "public-split")
      ~config
      ~style:
        (Style.create_exn [ Width (Length.px_exn 300.); Height (Length.px_exn 120.) ])
      ~panels:
        (List.map names ~f:(fun name ->
           id name, View.button ~on_click:(fun () -> ()) name))
      ~handles:[ id "a", View.text "Grip" ]
      ~on_resize:(fun _ -> ())
      ()
    |> ok
  in
  let actual =
    List.map
      [ Some (view [ "a"; "b"; "c" ] ~hidden:None ~generation:0L ~resize:None)
      ; Some (view [ "c"; "a"; "b" ] ~hidden:None ~generation:0L ~resize:None)
      ; Some (view [ "c"; "a"; "b" ] ~hidden:(Some "b") ~generation:0L ~resize:None)
      ; Some
          (view
             [ "c"; "a"; "b" ]
             ~hidden:None
             ~generation:1L
             ~resize:
               (Some
                  (Split_group.Resize_request.create (id "a") ~size:130. ~serial:1L |> ok)))
      ; Some (view [ "c"; "a"; "b" ] ~hidden:None ~generation:1L ~resize:None)
      ; None
      ]
      ~f:(fun view ->
        let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
        Reconciler.accept r update |> ok;
        hex Wire.Message.bin_writer_t (Reconciler.message update |> Option.value_exn))
    |> String.concat ~sep:"\n"
  in
  let expected = fixture "split-group-public.hex" in
  if not (String.equal actual expected) then print_endline actual;
  assert (String.equal actual expected);
  [%expect {| |}]
;;

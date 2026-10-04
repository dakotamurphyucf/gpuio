open Core
open Gpuio
module W = Gpuio_protocol.Calendar_content_wire

let ok = Or_error.ok_exn

let%expect_test "calendar slots preserve civil and pane identity" =
  let first = Calendar.Month.create ~year:1 ~month:Jan |> ok in
  let last = Calendar.Month.create ~year:9999 ~month:Dec |> ok in
  let slots =
    [ Calendar.Slot.previous
    ; Calendar.Slot.next
    ; Calendar.Slot.choose_month
    ; Calendar.Slot.choose_year
    ; Calendar.Slot.today
    ; Calendar.Slot.clear
    ; Calendar.Slot.day Calendar.min_date |> ok
    ; Calendar.Slot.day Calendar.max_date |> ok
    ; Calendar.Slot.month Jan
    ; Calendar.Slot.month Dec
    ; Calendar.Slot.year 1 |> ok
    ; Calendar.Slot.year 9999 |> ok
    ; Calendar.Slot.month_heading first
    ; Calendar.Slot.month_heading last
    ; Calendar.Slot.weekday first ~day:Sun
    ; Calendar.Slot.weekday last ~day:Sat
    ]
  in
  List.iter slots ~f:(fun slot ->
    assert (
      Calendar.Slot.equal
        slot
        (Calendar.Expert.slot_to_wire slot |> Calendar.Expert.slot_of_wire |> ok)));
  assert (
    List.length (List.dedup_and_sort slots ~compare:Calendar.Slot.compare)
    = List.length slots);
  assert (
    List.length
      (List.dedup_and_sort
         (List.map slots ~f:Calendar.Expert.slot_key)
         ~compare:String.compare)
    = List.length slots);
  List.iter [ Int.min_value; -1; 0; 10000; Int.max_value ] ~f:(fun year ->
    assert (Or_error.is_error (Calendar.Slot.year year)));
  assert (Or_error.is_error (Calendar.Slot.day (Date.create_exn ~y:0 ~m:Jan ~d:1)));
  List.iter
    W.Slot.
      [ Day (-1L)
      ; Day 3652059L
      ; Month 0L
      ; Month 13L
      ; Year 0L
      ; Year 10000L
      ; Month_heading (-1L)
      ; Month_heading 119988L
      ; Weekday (0L, -1L)
      ; Weekday (0L, 7L)
      ; Weekday (119988L, 0L)
      ]
    ~f:(fun slot -> assert (Or_error.is_error (Calendar.Expert.slot_of_wire slot)));
  print_endline
    "civil bounds, slot round trips and distinct pane-qualified identities pass";
  [%expect
    {| civil bounds, slot round trips and distinct pane-qualified identities pass |}]
;;

let item slot description : W.Item.t = { slot; description }

module Content = View.Calendar_content

let content_item slot text = Content.Item.create ~slot (View.text text) |> ok

let%expect_test "calendar content checks passive structure before accepting a collection" =
  assert (
    Or_error.is_error
      (Content.Item.create
         ~slot:Calendar.Slot.next
         (View.button ~on_click:(fun () -> ()) "Nested action")));
  assert (
    Or_error.is_error
      (Content.Item.create ~description:"\n" ~slot:Calendar.Slot.next (View.text "Next")));
  let next = content_item Calendar.Slot.next "Next" in
  assert (Or_error.is_error (Content.create [ next; next ]));
  assert (Or_error.is_error (Content.create (List.init 1025 ~f:(fun _ -> next))));
  let deep =
    List.fold (List.init 127 ~f:Fn.id) ~init:(View.text "deep") ~f:(fun v _ ->
      View.column [ v ])
  in
  let item = Content.Item.create ~slot:Calendar.Slot.next deep |> ok in
  assert (Or_error.is_error (Content.create [ item ]));
  let wide = View.column (List.init 2046 ~f:(fun _ -> View.text "one")) in
  let a = Content.Item.create ~slot:Calendar.Slot.previous wide |> ok in
  let b = Content.Item.create ~slot:Calendar.Slot.next wide |> ok in
  assert (Or_error.is_ok (Content.create [ a; b ]));
  let c = content_item Calendar.Slot.today "one more" in
  assert (Or_error.is_error (Content.create [ a; b; c ]));
  print_endline
    "callbacks, duplicates, count, wrapper depth and aggregate nodes checked early";
  [%expect
    {| callbacks, duplicates, count, wrapper depth and aggregate nodes checked early |}]
;;

let%expect_test
    "rich calendar slots preserve owner and keys across edits and supplied order"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let config = Calendar.Config.create ~label:"Dates" () |> ok in
  let initial_month = Calendar.Month.create ~year:2024 ~month:Feb |> ok in
  let view items =
    View.calendar
      ~content:(Content.create items |> ok)
      ~controller:(Key.of_string_exn "calendar")
      ~config
      ~initial:Calendar.Selection.empty
      ~initial_month
      ~on_event:ignore
      ()
  in
  let commit view =
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Gpuio_protocol.Wire.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let previous = content_item Calendar.Slot.previous "Back" in
  let next = content_item Calendar.Slot.next "Forward" in
  let initial = commit (view [ next; previous ]) in
  assert (
    List.count initial ~f:(function
      | Gpuio_protocol.Wire.Op.Set_calendar_content _ -> true
      | _ -> false)
    = 1);
  assert (List.is_empty (commit (view [ previous; next ])));
  let changed = commit (view [ previous; content_item Calendar.Slot.next "Later" ]) in
  assert (
    List.exists changed ~f:(function
      | Gpuio_protocol.Wire.Op.Set_text (_, "Later") -> true
      | _ -> false));
  assert (
    not
      (List.exists changed ~f:(function
         | Gpuio_protocol.Wire.Op.Create _
         | Remove _
         | Set_calendar _
         | Set_calendar_content _ -> true
         | _ -> false)));
  let removed = commit (view []) in
  assert (
    not
      (List.exists removed ~f:(function
         | Gpuio_protocol.Wire.Op.Set_calendar _ -> true
         | _ -> false)));
  print_endline
    "canonical order is silent; descendant edit preserves slots and native owner; empty \
     resets content";
  [%expect
    {| canonical order is silent; descendant edit preserves slots and native owner; empty resets content |}]
;;

let%expect_test "slot metadata uses independent bytes and bounded validated descriptions" =
  let items =
    [ item Previous (Some "Back")
    ; item Next None
    ; item Choose_month None
    ; item Choose_year None
    ; item Today None
    ; item Clear None
    ; item (Day 0L) (Some "Holiday")
    ; item (Month 12L) None
    ; item (Year 9999L) None
    ; item (Month_heading 0L) None
    ; item (Weekday (119987L, 6L)) (Some "Weekend")
    ]
  in
  assert (W.valid items);
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let message =
    Gpuio_protocol.Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_calendar_content (node, Some items); Set_calendar_content (node, None) ]
      }
  in
  let op_bytes = Gpuio_protocol.Wire.Message.encode message |> ok in
  let op_hex =
    String.to_list op_bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "calendar-content-operation.hex")
      |> String.strip
    in
    assert (String.equal expected op_hex));
  let encoded = Bin_prot.Utils.bin_dump W.bin_writer_t items |> Bigstring.to_string in
  let hex =
    String.to_list encoded
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "calendar-content.hex") |> String.strip
    in
    assert (String.equal hex expected));
  assert (not (W.valid (List.hd_exn items :: items)));
  assert (not (W.valid [ item Next None; item Previous None ]));
  List.iter
    [ ""; "   "; "\n"; "\000"; "\127"; "\255"; String.make 1025 'x' ]
    ~f:(fun text -> assert (not (W.valid [ item Previous (Some text) ])));
  assert (W.valid [ item Previous (Some "é🎉") ]);
  let many n description =
    List.init n ~f:(fun i -> item (Day (Int64.of_int i)) description)
  in
  assert (W.valid (many 1024 None));
  assert (not (W.valid (many 1025 None)));
  let full = many 64 (Some (String.make 1024 'x')) in
  assert (W.valid full);
  assert (not (W.valid (full @ [ item (Day 64L) (Some "x") ])));
  print_endline
    "11 slot tags agree; duplicates, UTF-8, count and aggregate-byte boundaries pass";
  [%expect
    {| 11 slot tags agree; duplicates, UTF-8, count and aggregate-byte boundaries pass |}]
;;

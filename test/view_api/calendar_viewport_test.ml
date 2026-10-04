open Core
open Gpuio
open Gpuio_protocol
module P = Calendar_viewport_wire

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok
let node = Node_id.create ~slot:1L ~generation:2L |> ok
let handler = Handler_id.create ~slot:3L ~generation:4L |> ok
let month = Calendar.Month.create ~year:2024 ~month:Month.Feb |> ok

let display =
  P.Display.Days
    { first_month = Calendar.Expert.month_to_wire month; months = 2L; first_weekday = 1L }
;;

let sample sequence = P.{ sequence; display }

let encode events =
  Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
;;

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let fixture name =
  Eio_main.run (fun env ->
    Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip)
;;

let viewport ?(observer = handler) sample =
  Calendar.Expert.viewport_of_wire ~window ~node ~observer sample |> ok
;;

let%expect_test "viewport covers distinct civil grid dates, not the selection cursor" =
  let view = viewport (sample 0L) in
  assert (
    List.equal
      Calendar.Month.equal
      (Calendar.Viewport.months view)
      [ month; Calendar.Month.shift month ~months:1 |> ok ]);
  let dates = Calendar.Viewport.dates view in
  assert (List.is_sorted_strictly dates ~compare:Date.compare);
  print_s [%sexp (List.hd_exn dates : Date.t), (List.last_exn dates : Date.t)];
  List.iter [ 0L; 119976L ] ~f:(fun first_month ->
    let v =
      viewport
        P.
          { sequence = 0L
          ; display = Days { first_month; months = 12L; first_weekday = 0L }
          }
    in
    assert (List.length (Calendar.Viewport.months v) = 12);
    assert (List.length (Calendar.Viewport.dates v) <= 504);
    List.iter (Calendar.Viewport.dates v) ~f:(fun date ->
      assert (Result.is_ok (Calendar.Expert.date_to_ordinal date))));
  List.iter
    [ P.Display.Months { year = 2024L }; Years { first = 9981L; last = 9999L } ]
    ~f:(fun display ->
      let v = viewport P.{ sequence = 0L; display } in
      assert (List.is_empty (Calendar.Viewport.months v));
      assert (List.is_empty (Calendar.Viewport.dates v)));
  assert (not (Calendar.Viewport.equal view (viewport (sample 1L))));
  assert (
    Calendar.Viewport.Display.equal
      (Calendar.Viewport.display view)
      (Calendar.Viewport.display (viewport (sample 1L))));
  [%expect {| (2024-01-29 2024-04-07) |}]
;;

let%expect_test
    "calendar viewport subscription generations reject late events and preserve the main \
     callback"
  =
  let r = Reconciler.create window in
  let view ?on_viewport_change () =
    View.calendar
      ?on_viewport_change
      ~controller:(Key.of_string_exn "calendar")
      ~config:(Calendar.Config.create ~label:"Dates" () |> ok)
      ~initial:Calendar.Selection.empty
      ~initial_month:month
      ~on_event:(fun _ -> "selection")
      ()
  in
  let prepare v = Reconciler.prepare r ~theme:Theme.default v |> ok in
  let accept u =
    Reconciler.accept r u |> ok;
    match Reconciler.message u with
    | Some (Apply tx) -> tx.operations
    | _ -> []
  in
  let mounted =
    accept (prepare (Some (view ~on_viewport_change:(fun _ -> "first") ())))
  in
  let node, main =
    List.find_map_exn mounted ~f:(function
      | Wire.Op.Create (n, Calendar, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let observer ops =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Set_calendar_viewport_observer (_, Some h) -> Some h
      | _ -> None)
  in
  let first = observer mounted in
  assert (not (Handler_id.equal first main));
  let event ?(revision = 1L) h sequence =
    Wire.Event.Calendar_viewport_changed (window, node, h, revision, sample sequence)
  in
  let dispatch expected e =
    assert (Option.equal String.equal (Reconciler.dispatch r e) expected)
  in
  dispatch (Some "first") (event first 0L);
  dispatch None (event first 0L);
  dispatch None (event main 1L);
  dispatch None (event ~revision:100L first 1L);
  let pending = prepare (Some (view ~on_viewport_change:(fun _ -> "latest") ())) in
  dispatch (Some "first") (event first 1L);
  assert (List.is_empty (accept pending));
  dispatch (Some "latest") (event first 2L);
  ignore (accept (prepare (Some (view ()))) : Wire.Op.t list);
  dispatch None (event first 3L);
  let restored =
    accept (prepare (Some (view ~on_viewport_change:(fun _ -> "restored") ())))
  in
  assert (
    not
      (List.exists restored ~f:(function
         | Wire.Op.Create _ | Bind _ | Remove _ -> true
         | _ -> false)));
  let next = observer restored in
  assert (not (Handler_id.equal first next));
  dispatch None (event first 999L);
  dispatch (Some "restored") (event ~revision:3L next 0L);
  dispatch
    (Some "selection")
    (Wire.Event.Calendar_event
       ( window
       , node
       , main
       , 3L
       , Calendar_wire.Event.Observed
           { revision = 0L
           ; mode = Single
           ; selection = Empty
           ; selection_allowed = true
           ; month = Calendar.Expert.month_to_wire month
           ; focused_date =
               Calendar.Expert.date_to_ordinal (Calendar.Month.first_day month) |> ok
           ; presentation = Days
           ; focused = false
           } ));
  let valid = sample 1L in
  dispatch
    None
    (Wire.Event.Calendar_viewport_changed
       (window, node, next, 3L, { valid with display = Months { year = 0L } }));
  dispatch (Some "restored") (event ~revision:3L next 1L);
  ignore (accept (prepare None) : Wire.Op.t list);
  dispatch None (event next 2L);
  let remounted =
    accept (prepare (Some (view ~on_viewport_change:(fun _ -> "new owner") ())))
  in
  let new_node =
    List.find_map_exn remounted ~f:(function
      | Wire.Op.Create (node, Calendar, _, _) -> Some node
      | _ -> None)
  in
  let new_handler = observer remounted in
  let current sequence =
    Wire.Event.Calendar_viewport_changed
      (window, new_node, new_handler, Reconciler.revision r, sample sequence)
  in
  dispatch (Some "new owner") (current 0L);
  Reconciler.close r;
  dispatch None (current 1L);
  print_endline
    "latest accepted callback; independent subscription; duplicate/stale/malformed \
     events rejected";
  [%expect
    {| latest accepted callback; independent subscription; duplicate/stale/malformed events rejected |}]
;;

let%expect_test "calendar viewport wire validates every display boundary" =
  let events =
    List.mapi
      [ display; Months { year = 1L }; Years { first = 9981L; last = 9999L } ]
      ~f:(fun sequence display ->
        Wire.Event.Calendar_viewport_changed
          (window, node, handler, 5L, P.{ sequence = Int64.of_int sequence; display }))
  in
  let bytes = encode events in
  assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) events);
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ P.Display.Days { first_month = 119987L; months = 2L; first_weekday = 0L }
    ; Days { first_month = 0L; months = 0L; first_weekday = 0L }
    ; Days { first_month = 0L; months = 13L; first_weekday = 0L }
    ; Days { first_month = 0L; months = 1L; first_weekday = 7L }
    ; Months { year = 10000L }
    ; Years { first = 2020L; last = 2039L }
    ; Years { first = 2021L; last = 2039L }
    ]
    ~f:(fun display ->
      assert (
        Result.is_error
          (Wire.Event.decode
             (encode
                [ Calendar_viewport_changed
                    (window, node, handler, 5L, P.{ sequence = 0L; display })
                ]))));
  List.iter
    [ -1L, 0L; 0L, -1L ]
    ~f:(fun (revision, sequence) ->
      assert (
        Result.is_error
          (Wire.Event.decode
             (encode
                [ Calendar_viewport_changed
                    (window, node, handler, revision, sample sequence)
                ]))));
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_calendar_viewport_observer (node, Some handler)
          ; Set_calendar_viewport_observer (node, None)
          ]
      }
  in
  assert (
    String.equal
      (hex
         (Bin_prot.Utils.bin_dump Wire.Message.bin_writer_t message |> Bigstring.to_string))
      (fixture "calendar-viewport-operation.hex"));
  assert (String.equal (hex bytes) (fixture "calendar-viewport-events.hex"));
  [%expect {| |}]
;;

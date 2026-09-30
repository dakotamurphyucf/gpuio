open Core
module C = Gpuio.Calendar

let ok = Or_error.ok_exn

let selection_ok result =
  Result.map_error result ~f:(fun error ->
    Error.create_s (C.Selection_error.sexp_of_t error))
  |> ok
;;

let date = Date.of_string
let range first last = C.Range.create ~first:(date first) ~last:(date last) |> ok
let selected first last = C.Selection.range (range first last)

let%expect_test "bridge ordinals agree with independent civil reference vectors" =
  List.iter
    [ "0001-01-01", 0L, 1
    ; "1600-02-29", 584081L, 2
    ; "1900-03-01", 693654L, 4
    ; "1970-01-01", 719162L, 4
    ; "2000-01-01", 730119L, 6
    ; "2000-02-29", 730178L, 2
    ; "2024-02-29", 738944L, 4
    ; "2100-03-01", 766703L, 1
    ; "2400-02-29", 876275L, 2
    ; "9999-12-31", 3652058L, 5
    ]
    ~f:(fun (text, ordinal, weekday) ->
      let day = date text in
      assert (Int64.equal (C.Expert.date_to_ordinal day |> ok) ordinal);
      assert (Date.equal (C.Expert.date_of_ordinal ordinal |> ok) day);
      assert (Day_of_week.to_int (Date.day_of_week day) = weekday));
  List.iter [ Int64.min_value; -1L; 3652059L; Int64.max_value ] ~f:(fun ordinal ->
    assert (Result.is_error (C.Expert.date_of_ordinal ordinal)));
  assert (Result.is_error (C.Expert.date_to_ordinal (Date.create_exn ~y:0 ~m:Jan ~d:1)));
  print_endline
    "ten independent ordinal/weekday vectors agree; out-of-domain wire dates reject \
     before conversion";
  [%expect
    {| ten independent ordinal/weekday vectors agree; out-of-domain wire dates reject before conversion |}]
;;

let%expect_test "civil limits, ordered ranges and overflow-safe month navigation" =
  List.iter [ "0001-01-01"; "9999-12-31"; "2000-02-29" ] ~f:(fun text ->
    assert (C.is_supported_date (date text));
    assert (Result.is_ok (C.Selection.single (date text))));
  let before = Date.create_exn ~y:0 ~m:Dec ~d:31 in
  assert (not (C.is_supported_date before));
  assert (Result.is_error (C.Selection.single before));
  assert (Result.is_error (C.Selection.range_start before));
  assert (Result.is_error (C.Month.of_date before));
  assert (Result.is_error (C.Range.create ~first:before ~last:C.max_date));
  assert (Result.is_error (C.Range.create ~first:C.max_date ~last:C.min_date));
  let first = C.Month.of_date C.min_date |> ok in
  let last = C.Month.of_date C.max_date |> ok in
  assert (C.Month.equal (C.Month.shift first ~months:119987 |> ok) last);
  assert (C.Month.equal (C.Month.shift last ~months:(-119987) |> ok) first);
  List.iter [ Int.min_value; -1 ] ~f:(fun months ->
    assert (Result.is_error (C.Month.shift first ~months)));
  List.iter [ 1; Int.max_value ] ~f:(fun months ->
    assert (Result.is_error (C.Month.shift last ~months)));
  let december = C.Month.create ~year:2023 ~month:Dec |> ok in
  let january = C.Month.shift december ~months:1 |> ok in
  print_s
    [%sexp (C.Month.first_day january : Date.t), (C.Month.last_day january : Date.t)];
  print_s [%sexp (Date.diff C.max_date C.min_date : int)];
  [%expect
    {|
    (2024-01-01 2024-01-31)
    3652058
    |}]
;;

let%expect_test "month grids across a Gregorian cycle and every first weekday" =
  for year = 2000 to 2399 do
    for month = 1 to 12 do
      let month = C.Month.create ~year ~month:(Month.of_int_exn month) |> ok in
      List.iter Day_of_week.all ~f:(fun first_weekday ->
        let weeks = C.Month.weeks month ~first_weekday in
        assert (List.length weeks = 6);
        List.iter weeks ~f:(fun week -> assert (List.length week = 7));
        let days = List.concat weeks |> List.map ~f:(fun day -> Option.value_exn day) in
        assert (Day_of_week.equal (Date.day_of_week (List.hd_exn days)) first_weekday);
        List.iter
          (List.zip_exn (List.drop_last_exn days) (List.tl_exn days))
          ~f:(fun (a, b) -> assert (Date.diff b a = 1));
        let own_days =
          List.filter days ~f:(fun day ->
            Date.compare day (C.Month.first_day month) >= 0
            && Date.compare day (C.Month.last_day month) <= 0)
        in
        assert (Date.equal (List.hd_exn own_days) (C.Month.first_day month));
        assert (Date.equal (List.last_exn own_days) (C.Month.last_day month));
        assert (List.length own_days = Date.day (C.Month.last_day month)))
    done
  done;
  List.iter [ C.min_date; C.max_date ] ~f:(fun boundary ->
    let month = C.Month.of_date boundary |> ok in
    List.iter Day_of_week.all ~f:(fun first_weekday ->
      let cells = C.Month.weeks month ~first_weekday |> List.concat in
      let days = List.filter_opt cells in
      assert (List.length cells = 42);
      assert (List.length days < 42 || Date.equal boundary C.min_date);
      assert (List.for_all days ~f:C.is_supported_date);
      assert (List.mem days boundary ~equal:Date.equal)));
  print_endline
    "33600 grids: fixed bounds, consecutive days, correct weekdays and complete months; \
     civil edges do not wrap";
  [%expect
    {| 33600 grids: fixed bounds, consecutive days, correct weekdays and complete months; civil edges do not wrap |}]
;;

let%expect_test "constraint normalization and input bounds" =
  let t =
    C.Constraints.create
      ~disabled_dates:(List.map [ "2024-01-03"; "2024-01-01"; "2024-01-03" ] ~f:date)
      ~disabled_ranges:
        [ range "2024-01-03" "2024-01-06"
        ; range "2024-01-01" "2024-01-02"
        ; range "2024-01-05" "2024-01-10"
        ]
      ~disabled_weekdays:[ Sat; Sun; Sat ]
      ()
    |> ok
  in
  print_s [%sexp (C.Constraints.disabled_dates t : Date.t list)];
  print_s [%sexp (C.Constraints.disabled_ranges t : C.Range.t list)];
  print_s [%sexp (C.Constraints.disabled_weekdays t : Day_of_week.t list)];
  let duplicate = date "2024-01-01" in
  let cases =
    [ C.Constraints.create ~disabled_dates:(List.init 513 ~f:(fun _ -> duplicate)) ()
    ; C.Constraints.create
        ~disabled_ranges:(List.init 129 ~f:(fun _ -> range "2024-01-01" "2024-01-01"))
        ()
    ; C.Constraints.create
        ~disabled_weekdays:(List.init 8 ~f:(fun _ -> Day_of_week.Sun))
        ()
    ; C.Constraints.create ~min:C.max_date ~max:C.min_date ()
    ; C.Constraints.create ~disabled_dates:[ Date.create_exn ~y:0 ~m:Jan ~d:1 ] ()
    ]
  in
  assert (List.for_all cases ~f:Result.is_error);
  let disabled = C.Constraints.create ~disabled_weekdays:Day_of_week.all () |> ok in
  assert (not (C.Constraints.allows disabled duplicate));
  assert (C.Constraints.allows_selection disabled C.Selection.empty ~mode:Single);
  assert (C.Constraints.allows_selection disabled C.Selection.empty ~mode:Range);
  [%expect
    {|
    (2024-01-01 2024-01-03)
    (((first 2024-01-01) (last 2024-01-10)))
    (SUN SAT)
    |}]
;;

let%expect_test "partial range activation, restart, completion and rejection" =
  let constraints = C.Constraints.create ~disabled_dates:[ date "2024-04-02" ] () |> ok in
  let show t = print_s [%sexp (t : C.Selection.t)] in
  let click t day = C.activate t ~date:(date day) ~mode:Range ~constraints in
  let start = click C.Selection.empty "2024-04-03" |> selection_ok in
  show start;
  let earlier = click start "2024-04-01" |> selection_ok in
  show earlier;
  print_s
    [%sexp (click earlier "2024-04-03" : (C.Selection.t, C.Selection_error.t) Result.t)];
  let same = click earlier "2024-04-01" |> selection_ok in
  show same;
  assert (C.Selection.is_complete same);
  show (click same "2024-04-03" |> selection_ok);
  print_s
    [%sexp (click earlier "2024-04-02" : (C.Selection.t, C.Selection_error.t) Result.t)];
  let single = C.Selection.single (date "2024-04-01") |> ok in
  print_s
    [%sexp (click single "2024-04-03" : (C.Selection.t, C.Selection_error.t) Result.t)];
  let endpoints =
    C.Constraints.create
      ~disabled_dates:[ date "2024-04-02" ]
      ~range_policy:Endpoints_only
      ()
    |> ok
  in
  let allowed =
    C.activate earlier ~date:(date "2024-04-03") ~mode:Range ~constraints:endpoints
    |> selection_ok
  in
  show allowed;
  assert (not (C.Constraints.allows_selection constraints allowed ~mode:Range));
  show allowed;
  [%expect
    {|
    (Range_start 2024-04-03)
    (Range_start 2024-04-01)
    (Error Disabled_interior)
    (Range ((first 2024-04-01) (last 2024-04-01)))
    (Range_start 2024-04-03)
    (Error Disabled_date)
    (Error Wrong_mode)
    (Range ((first 2024-04-01) (last 2024-04-03)))
    (Range ((first 2024-04-01) (last 2024-04-03)))
    |}]
;;

let%expect_test "bounded range policy agrees with a daily reference" =
  let first = date "2024-01-01" in
  List.iter [ C.Range_policy.Every_day; Endpoints_only ] ~f:(fun range_policy ->
    let constraints =
      C.Constraints.create
        ~min:(date "2024-01-02")
        ~max:(date "2024-03-10")
        ~disabled_dates:[ date "2024-01-09"; date "2024-02-01" ]
        ~disabled_ranges:
          [ range "2024-01-15" "2024-01-17"; range "2024-02-27" "2024-03-01" ]
        ~disabled_weekdays:[ Sat; Sun ]
        ~range_policy
        ()
      |> ok
    in
    for start = 0 to 70 do
      for finish = start to 70 do
        let a = Date.add_days first start
        and b = Date.add_days first finish in
        let selection = C.Selection.range (C.Range.create ~first:a ~last:b |> ok) in
        let reference =
          match range_policy with
          | Endpoints_only ->
            C.Constraints.allows constraints a && C.Constraints.allows constraints b
          | Every_day ->
            List.for_all
              (List.init (finish - start + 1) ~f:(Date.add_days a))
              ~f:(C.Constraints.allows constraints)
        in
        assert (
          Bool.equal
            reference
            (C.Constraints.allows_selection constraints selection ~mode:Range))
      done
    done);
  let all = selected "0001-01-01" "9999-12-31" in
  assert (C.Constraints.allows_selection C.Constraints.unrestricted all ~mode:Range);
  List.iter Day_of_week.all ~f:(fun weekday ->
    let constraints = C.Constraints.create ~disabled_weekdays:[ weekday ] () |> ok in
    assert (not (C.Constraints.allows_selection constraints all ~mode:Range)));
  print_endline
    "5112 interval checks agree with daily reference; full civil span checks disabled \
     weekdays without scanning days";
  [%expect
    {| 5112 interval checks agree with daily reference; full civil span checks disabled weekdays without scanning days |}]
;;

let%expect_test "strict date formats, leap centuries and Gregorian-cycle round trips" =
  List.iter
    [ "1900-02-29"
    ; "2100-02-29"
    ; "2024-04-31"
    ; "0000-01-01"
    ; "2024-00-01"
    ; "2024-13-01"
    ; "2024-01-00"
    ; "2024-1-01"
    ; " 2024-01-01"
    ; "2024-01-01\n"
    ; "２０２４-01-01"
    ; "2024/01/01"
    ]
    ~f:(fun text -> assert (Result.is_error (C.Format.parse Iso text)));
  List.iter [ "2000-02-29"; "2400-02-29"; "0001-01-01"; "9999-12-31" ] ~f:(fun text ->
    assert (Date.equal (C.Format.parse Iso text |> ok) (date text)));
  List.iter [ C.Format.Iso; Day_month_year; Month_day_year ] ~f:(fun format ->
    print_endline (C.Format.format format (date "2024-02-29") |> ok));
  print_s [%sexp (C.Format.parse Day_month_year "03/04/2024" |> ok : Date.t)];
  print_s [%sexp (C.Format.parse Month_day_year "03/04/2024" |> ok : Date.t)];
  let first = date "2000-01-01" in
  for offset = 0 to 146096 do
    let day = Date.add_days first offset in
    List.iter [ C.Format.Iso; Day_month_year; Month_day_year ] ~f:(fun format ->
      let formatted = C.Format.format format day |> ok in
      assert (Date.equal (C.Format.parse format formatted |> ok) day))
  done;
  print_endline "438291 exact format round trips across a complete Gregorian leap cycle";
  [%expect
    {|
    2024-02-29
    29/02/2024
    02/29/2024
    2024-04-03
    2024-03-04
    438291 exact format round trips across a complete Gregorian leap cycle
    |}]
;;

let%expect_test "calendar capability uses the shared 64-bit handshake" =
  let module Wire = Gpuio_protocol.Wire in
  assert (Int64.equal (Int64.bit_and Wire.capabilities 68719476736L) 68719476736L);
  let bytes =
    Wire.Message.encode (Hello (Wire.version, Wire.capabilities)) |> Or_error.ok_exn
  in
  String.iter bytes ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0001fcffffffffffff0300 |}]
;;

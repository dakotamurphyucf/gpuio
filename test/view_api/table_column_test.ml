open Core
open Gpuio
module C = Table_column
module Schema = C.Collection

let ok = Or_error.ok_exn
let id text = C.Id.of_string text |> ok

let column ?pin ?movable ?resizable name =
  C.create ~id:(id name) ~label:name ?pin ?movable ?resizable () |> ok
;;

let group label names = C.Group.create ~label ~columns:(List.map names ~f:id) |> ok

let names schema =
  List.map (Schema.to_list schema) ~f:(fun column -> C.Id.to_string (C.id column))
;;

let error result = print_s [%sexp (result |> Or_error.map ~f:ignore : unit Or_error.t)]

let%expect_test "column boundaries protect both application sizes and native proposals" =
  let name = id "計算👨‍👩‍👧‍👦" in
  let original =
    C.create ~id:name ~label:"計算" ~width:100. ~min_width:50. ~max_width:200. () |> ok
  in
  let schema = Schema.create [ original ] |> ok in
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity ] ~f:(fun width ->
    assert (Result.is_error (C.with_width original width));
    assert (Result.is_error (Schema.resize schema ~column:name ~width)));
  List.iter [ 0.; 49.; 201. ] ~f:(fun width ->
    assert (Result.is_error (C.with_width original width)));
  List.iter [ -100.; 125.; 1000. ] ~f:(fun width ->
    let resized = Schema.resize schema ~column:name ~width |> ok in
    let column = Schema.find resized name |> Option.value_exn in
    print_s [%sexp (C.width column : float), (C.Id.equal name (C.id column) : bool)]);
  assert (Float.equal (C.width original) 100.);
  error (C.create ~id:name ~label:"invalid" ~min_width:19. ());
  error (C.create ~id:name ~label:"invalid" ~max_width:16385. ());
  let fixed = column ~resizable:false "fixed" in
  assert (Float.equal (C.width (C.with_width fixed 200. |> ok)) 200.);
  error (Schema.resize (Schema.create [ fixed ] |> ok) ~column:(id "fixed") ~width:200.);
  error (Schema.resize schema ~column:(id "missing") ~width:100.);
  [%expect
    {|
    (50 true)
    (125 true)
    (200 true)
    (Error "table widths require finite 20 <= min <= width <= max <= 16384")
    (Error "table widths require finite 20 <= min <= width <= max <= 16384")
    (Error "table column is not user-resizable")
    (Error "unknown table column: missing")
    |}]
;;

let%expect_test "labels and IDs validate UTF-8 without changing identity" =
  List.iter
    [ ""; "a\000b"; "\255"; String.make 257 'x' ]
    ~f:(fun text -> assert (Result.is_error (C.Id.of_string text)));
  List.iter
    [ ""; "a\000b"; "\255"; String.make 4097 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (C.create ~id:(id "a") ~label ()));
      assert (Result.is_error (C.Group.create ~label ~columns:[ id "a" ])));
  assert (not (C.Id.equal (id "A") (id "a")));
  assert (not (C.Id.equal (id "é") (id "é")));
  error (C.Group.create ~label:"empty" ~columns:[]);
  error (C.Group.create ~label:"duplicate" ~columns:[ id "a"; id "a" ]);
  error (Schema.create [ column "a"; column "a" ]);
  [%expect
    {|
    (Error "table group requires 1..64 columns")
    (Error "duplicate table group column")
    (Error "duplicate table column id")
    |}]
;;

let%expect_test "resize and reorder preserve pinned groups by stable membership" =
  let columns =
    [ column ~pin:Left "id"; column "a"; column "b"; column "c"; column "d" ]
  in
  let header_groups =
    [ [ group "Key" [ "id" ]; group "Values" [ "a"; "b"; "c"; "d" ] ]
    ; [ group "Key" [ "id" ]; group "Inputs" [ "a"; "b" ]; group "Outputs" [ "c"; "d" ] ]
    ]
  in
  let initial = Schema.create ~header_groups columns |> ok in
  let reordered = Schema.move initial ~column:(id "b") ~before:(Some (id "a")) |> ok in
  let resized = Schema.resize reordered ~column:(id "b") ~width:220. |> ok in
  print_s [%sexp (names initial : string list), (names resized : string list)];
  print_s [%sexp (Schema.header_groups resized : C.Group.t list list)];
  print_s
    [%sexp (Schema.pinned_count resized : int), (Schema.total_width resized : float)];
  error (Schema.move initial ~column:(id "b") ~before:(Some (id "d")));
  error (Schema.move initial ~column:(id "a") ~before:(Some (id "id")));
  error (Schema.move initial ~column:(id "id") ~before:None);
  assert (
    Schema.equal
      initial
      (Schema.move initial ~column:(id "b") ~before:(Some (id "b")) |> ok));
  [%expect
    {|
    ((id a b c d) (id b a c d))
    ((((label Key) (columns (id))) ((label Values) (columns (b a c d))))
     (((label Key) (columns (id))) ((label Inputs) (columns (b a)))
      ((label Outputs) (columns (c d)))))
    (1 860)
    (Error "table header level must partition columns in display order")
    (Error "left-pinned table columns must form a prefix")
    (Error "left-pinned table columns must form a prefix")
    |}]
;;

let%expect_test "malformed group partitions fail instead of assigning unrelated headers" =
  let columns = List.map [ "a"; "b"; "c"; "d" ] ~f:column in
  List.iter
    [ [ [] ]
    ; [ [ group "Missing" [ "a"; "b"; "c" ] ] ]
    ; [ [ group "Foreign" [ "a"; "b"; "c"; "missing" ] ] ]
    ; [ [ group "Wrong order" [ "b"; "a"; "c"; "d" ] ] ]
    ; [ [ group "Left" [ "a"; "b" ]; group "Right" [ "c"; "d" ] ]
      ; [ group "Crossing" [ "a"; "b"; "c" ]; group "Last" [ "d" ] ]
      ]
    ]
    ~f:(fun header_groups -> error (Schema.create ~header_groups columns));
  error
    (Schema.create
       ~header_groups:[ [ group "Crossing pin" [ "a"; "b" ] ] ]
       [ column ~pin:Left "a"; column "b" ]);
  error (Schema.create ~header_groups:[ [] ] []);
  assert (List.is_empty (Schema.to_list (Schema.create [] |> ok)));
  [%expect
    {|
    (Error "table header level must partition columns in display order")
    (Error "table header level must partition columns in display order")
    (Error "table header level must partition columns in display order")
    (Error "table header level must partition columns in display order")
    (Error "table header levels must refine their parent groups")
    (Error "table header group crosses the pinned boundary")
    (Error "table header level must partition columns in display order")
    |}]
;;

let%expect_test
    "all destinations preserve identity and relative order at the column limit"
  =
  let columns = List.init Schema.max_columns ~f:(fun i -> column (Int.to_string i)) in
  let schema = Schema.create columns |> ok in
  let destinations = None :: List.map columns ~f:(fun column -> Some (C.id column)) in
  let moves = ref 0 in
  List.iter columns ~f:(fun source ->
    List.iter destinations ~f:(fun before ->
      let key = C.id source in
      let moved = Schema.move schema ~column:key ~before |> ok in
      let remaining t =
        List.filter (Schema.to_list t) ~f:(fun item -> not (C.Id.equal key (C.id item)))
      in
      assert (List.equal C.equal (remaining moved) (remaining schema));
      assert (List.length (Schema.to_list moved) = Schema.max_columns);
      assert (C.equal (Schema.find moved key |> Option.value_exn) source);
      (match before with
       | None ->
         assert (
           Option.equal Int.equal (Schema.index moved key) (Some (Schema.max_columns - 1)))
       | Some destination ->
         if not (C.Id.equal key destination)
         then
           assert (
             (Schema.index moved key |> Option.value_exn) + 1
             = (Schema.index moved destination |> Option.value_exn)));
      incr moves));
  print_s [%sexp (!moves : int)];
  error (Schema.create (columns @ [ column "overflow" ]));
  let groups = [ group "all" (List.map columns ~f:(fun c -> C.Id.to_string (C.id c))) ] in
  ignore
    (Schema.create ~header_groups:(List.init 4 ~f:(fun _ -> groups)) columns |> ok
     : Schema.t);
  error (Schema.create ~header_groups:(List.init 5 ~f:(fun _ -> groups)) columns);
  let oversized =
    List.init 64 ~f:(fun i ->
      C.create ~id:(id (Int.to_string i)) ~label:(String.make 4096 'x') () |> ok)
  in
  error (Schema.create oversized);
  let locked = Schema.create [ column ~movable:false "locked"; column "other" ] |> ok in
  error (Schema.move locked ~column:(id "locked") ~before:None);
  let moved =
    Schema.move locked ~column:(id "other") ~before:(Some (id "locked")) |> ok
  in
  print_s [%sexp (names moved : string list)];
  error (Schema.move schema ~column:(id "missing") ~before:None);
  error (Schema.move schema ~column:(id "0") ~before:(Some (id "missing")));
  [%expect
    {|
    4160
    (Error "table exceeds 64 columns")
    (Error "table exceeds 4 group header levels")
    (Error "table schema exceeds 262144 text bytes")
    (Error "table column is not user-movable")
    (other locked)
    (Error "unknown table column: missing")
    (Error "unknown table column: missing")
    |}]
;;

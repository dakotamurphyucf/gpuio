open Core
module D = Gpuio.Table_data

let page_size = 128
let max_pages = 4
let columns = 64
let id n = D.Id.of_string (Int.to_string n) |> Or_error.ok_exn

module Row = struct
  type t =
    { number : int
    ; cells : string array
    }

  let create number =
    { number
    ; cells =
        Array.init columns ~f:(fun column ->
          sprintf "%06d:%02d · 世界 · %s" number column (String.make 32 'x'))
    }
  ;;

  let number t = t.number
  let cell t ~column = t.cells.(column)
end

type t =
  { data : Row.t option D.t
  ; rows : int
  ; pages : Int.Set.t
  ; loaded_rows : int
  ; peak_loaded_rows : int
  ; loads : int
  ; evictions : int
  }

let data t = t.data
let loaded_rows t = t.loaded_rows
let peak_loaded_rows t = t.peak_loaded_rows
let loads t = t.loads
let evictions t = t.evictions

let create ~rows =
  if rows <= 0 || rows > 100_000
  then Or_error.error_string "qualification rows must be in 1..100000"
  else (
    let%map.Or_error data = D.create (List.init rows ~f:(fun n -> id n, None)) in
    { data
    ; rows
    ; pages = Int.Set.empty
    ; loaded_rows = 0
    ; peak_loaded_rows = 0
    ; loads = 0
    ; evictions = 0
    })
;;

let prepare t ~target =
  if target < 0 || target >= t.rows
  then Or_error.error_string "cache target is outside logical rows"
  else (
    let page = target / page_size in
    let page_count = (t.rows + page_size - 1) / page_size in
    let first = Int.max 0 (Int.min (page - 1) (page_count - max_pages)) in
    let pages =
      List.range first (Int.min page_count (first + max_pages)) |> Int.Set.of_list
    in
    let row_count page = Int.min page_size (t.rows - (page * page_size)) in
    let update_page data page ~load =
      List.range (page * page_size) (Int.min t.rows ((page + 1) * page_size))
      |> List.fold ~init:data ~f:(fun data n ->
        D.set data ~key:(id n) ~data:(if load then Some (Row.create n) else None)
        |> Or_error.ok_exn)
    in
    let removed = Set.diff t.pages pages in
    let added = Set.diff pages t.pages in
    let data = Set.fold removed ~init:t.data ~f:(update_page ~load:false) in
    let data = Set.fold added ~init:data ~f:(update_page ~load:true) in
    let loaded_rows = Set.sum (module Int) pages ~f:row_count in
    assert (loaded_rows <= max_pages * page_size);
    Ok
      { t with
        data
      ; pages
      ; loaded_rows
      ; peak_loaded_rows = Int.max t.peak_loaded_rows loaded_rows
      ; loads = t.loads + Set.length added
      ; evictions = t.evictions + Set.length removed
      })
;;

let%expect_test "disjoint paging preserves logical identity and evicts payloads" =
  let ok = Or_error.ok_exn in
  let initial = create ~rows:1030 |> ok in
  let first = prepare initial ~target:0 |> ok in
  let last = prepare first ~target:1029 |> ok in
  let middle = prepare last ~target:512 |> ok in
  let present t n = D.find t.data (id n) |> Option.join |> Option.is_some in
  let row_ref t n = D.row_ref t.data (id n) |> Option.value_exn in
  print_s
    [%sexp
      (loaded_rows first : int)
    , (loaded_rows last : int)
    , (peak_loaded_rows middle : int)
    , (present first 0 : bool)
    , (present last 0 : bool)
    , (present last 1029 : bool)
    , (D.same_source initial.data middle.data : bool)
    , (D.Row_ref.equal (row_ref initial 0) (row_ref last 0) : bool)
    , (D.length middle.data : int)
    , (loads middle : int)
    , (evictions middle : int)];
  [%expect {| (512 390 512 true false true true true 1030 10 6) |}];
  List.iter [ -1; 1030 ] ~f:(fun target ->
    assert (Result.is_error (prepare middle ~target)));
  let small = create ~rows:1 |> ok |> fun t -> prepare t ~target:0 |> ok in
  print_s [%sexp (loaded_rows small : int), (loads small : int), (evictions small : int)];
  [%expect {| (1 1 0) |}]
;;

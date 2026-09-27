open Core
module D = Gpuio.Table_data
module C = Gpuio.Table_column
module T = Gpuio.Table
module P = Gpuio.Table_paging

let ok = Or_error.ok_exn
let id number = D.Id.of_string (sprintf "result-%06d" number) |> ok
let column name = C.Id.of_string name |> ok

module Row = struct
  type t =
    { number : int
    ; tool : string
    ; score : int
    ; summary : string
    }

  let number t = t.number
  let tool t = t.tool
  let score t = t.score
  let summary t = t.summary
end

module Query = struct
  module Size = struct
    type t =
      | Sample
      | Large
    [@@deriving equal, sexp_of]
  end

  module Filter = struct
    type t =
      | All
      | High_score
      | Between of Score_range.t
      | Empty
    [@@deriving equal, sexp_of]
  end

  module Loading = struct
    type t =
      | Normal
      | Slow
      | Fail_once
    [@@deriving equal, sexp_of]
  end

  type t =
    { size : Size.t
    ; filter : Filter.t
    ; sort : T.Sort.t option
    ; loading : Loading.t
    }

  let validate_sort = function
    | None -> Ok ()
    | Some sort ->
      (match C.Id.to_string sort.T.Sort.column with
       | "number" | "tool" | "score" -> Ok ()
       | _ -> Or_error.error_string "This result column does not support sorting")
  ;;

  let create
        ?(size = Size.Sample)
        ?(filter = Filter.All)
        ?sort
        ?(loading = Loading.Normal)
        ()
    =
    let%map.Or_error () = validate_sort sort in
    { size; filter; sort; loading }
  ;;

  let size t = t.size
  let filter t = t.filter
  let sort t = t.sort
  let loading t = t.loading

  let with_sort t sort =
    let%map.Or_error () = validate_sort sort in
    { t with sort }
  ;;

  let with_filter t filter = { t with filter }
  let with_size t size = { t with size }
end

let schema columns =
  let pinned, scrolling =
    List.split_while columns ~f:(fun column -> C.Pin.equal (C.pin column) Left)
  in
  let%bind.Or_error groups =
    [ "RUN", pinned; "FINDINGS", scrolling ]
    |> List.filter_map ~f:(fun (label, columns) ->
      if List.is_empty columns
      then None
      else Some (C.Group.create ~label ~columns:(List.map columns ~f:C.id)))
    |> Or_error.all
  in
  C.Collection.create ~header_groups:[ groups ] columns
;;

let columns () =
  [ C.create
      ~id:(column "number")
      ~label:"RESULT"
      ~width:92.
      ~min_width:70.
      ~max_width:200.
      ~pin:Left
      ~sortable:true
      ()
    |> ok
  ; C.create
      ~id:(column "tool")
      ~label:"TOOL"
      ~width:126.
      ~min_width:90.
      ~sortable:true
      ()
    |> ok
  ; C.create
      ~id:(column "score")
      ~label:"SCORE"
      ~width:90.
      ~min_width:70.
      ~sortable:true
      ~alignment:Right
      ()
    |> ok
  ; C.create ~id:(column "summary") ~label:"FINDING" ~width:420. ~min_width:180. () |> ok
  ]
  |> schema
  |> ok
;;

let row number =
  { Row.number
  ; tool = (if number mod 2 = 0 then "Read source" else "Search")
  ; score = number * 37 mod 101
  ; summary = sprintf "Finding %06d · 日本語 · 👨‍👩‍👧‍👦" number
  }
;;

let rows query =
  let count =
    match Query.size query with
    | Sample -> 48
    | Large -> 100_000
  in
  let rows =
    List.init count ~f:(fun index -> row (index + 1))
    |> List.filter ~f:(fun row ->
      match Query.filter query with
      | All -> true
      | High_score -> row.Row.score >= 80
      | Between range -> Score_range.contains range row.Row.score
      | Empty -> false)
  in
  let compare left right =
    let primary =
      match Query.sort query with
      | None -> Int.compare left.Row.number right.Row.number
      | Some sort ->
        let comparison =
          match C.Id.to_string sort.column with
          | "tool" -> String.compare left.tool right.tool
          | "score" -> Int.compare left.score right.score
          | "number" -> Int.compare left.number right.number
          | _ -> assert false
        in
        (match sort.direction with
         | Ascending -> comparison
         | Descending -> -comparison)
    in
    if primary = 0 then Int.compare left.number right.number else primary
  in
  List.sort rows ~compare |> List.map ~f:(fun row -> id row.Row.number, row)
;;

let replace source query = D.replace source (rows query)

let page request =
  let open Or_error.Let_syntax in
  let%bind first =
    match P.Request.cursor request with
    | None -> Ok 0
    | Some cursor -> Or_error.try_with (fun () -> Int.of_string cursor)
  in
  if first < 0 || first > 100_000
  then Or_error.error_string "Invalid result page cursor"
  else (
    match P.Request.direction request with
    | Before -> Or_error.error_string "This sample pages forwards only"
    | After ->
      let all = rows (P.Request.query request) in
      let total = List.length all in
      if first > total
      then Or_error.error_string "Cursor exceeds the result query"
      else (
        let last = Int.min total (first + 24) in
        Ok
          { Gpuio_eio.Table_paging.Page.rows = List.sub all ~pos:first ~len:(last - first)
          ; next = (if last = total then End else More (Some (Int.to_string last)))
          }))
;;

let cell row column =
  match C.Id.to_string column with
  | "number" -> Ok (sprintf "%06d" row.Row.number)
  | "tool" -> Ok row.tool
  | "score" -> Ok (sprintf "%d%%" row.score)
  | "summary" -> Ok row.summary
  | _ -> Or_error.error_string "Unknown result column"
;;

let describe row =
  sprintf
    "Result %06d · %s · score %d%%\n%s"
    row.Row.number
    row.tool
    row.score
    row.summary
;;

open Core

let max_pages = 1_000_000_000

module Request = struct
  type t =
    | Page of int
    | First
    | Previous
    | Next
    | Last
  [@@deriving equal, sexp_of]

  let page page =
    if page < 1 || page > max_pages
    then Or_error.error_string "page must be in 1..1000000000"
    else Ok (Page page)
  ;;

  let first = First
  let previous = Previous
  let next = Next
  let last = Last
end

module Item = struct
  type t =
    | Page of int
    | Gap of
        { first : int
        ; last : int
        }
  [@@deriving equal, sexp_of]
end

type t =
  { total_pages : int
  ; current : int option
  ; siblings : int
  ; disabled : bool
  }
[@@deriving equal, sexp_of]

let validate_count total_pages =
  if total_pages < 0 || total_pages > max_pages
  then Or_error.error_string "page count must be in 0..1000000000"
  else Ok ()
;;

let valid_page ~total_pages page = page >= 1 && page <= total_pages

let create ~total_pages ?current ?(siblings = 1) ?(disabled = false) () =
  let open Or_error.Let_syntax in
  let%bind () = validate_count total_pages in
  let%bind () =
    if siblings < 0 || siblings > 4
    then Or_error.error_string "pagination siblings must be in 0..4"
    else Ok ()
  in
  let%map current =
    match current with
    | None -> Ok (if total_pages = 0 then None else Some 1)
    | Some page ->
      if valid_page ~total_pages page
      then Ok (Some page)
      else Or_error.error_string "selected page does not exist"
  in
  { total_pages; current; siblings; disabled }
;;

let total_pages t = t.total_pages
let current t = t.current
let siblings t = t.siblings
let is_disabled t = t.disabled
let with_disabled t disabled = { t with disabled }

let with_total_pages t total_pages =
  let%map.Or_error () = validate_count total_pages in
  let current =
    if total_pages = 0
    then None
    else Some (Int.min total_pages (Option.value t.current ~default:1))
  in
  { t with total_pages; current }
;;

let select t ~page =
  if valid_page ~total_pages:t.total_pages page
  then Ok { t with current = Some page }
  else Or_error.error_string "selected page does not exist"
;;

let apply_request t request =
  match t.current with
  | None -> t
  | Some current ->
    if t.disabled
    then t
    else (
      let page =
        match (request : Request.t) with
        | Page page -> page
        | First -> 1
        | Previous -> Int.max 1 (current - 1)
        | Next -> Int.min t.total_pages (current + 1)
        | Last -> t.total_pages
      in
      if valid_page ~total_pages:t.total_pages page
      then { t with current = Some page }
      else t)
;;

let items t =
  match t.current with
  | None -> []
  | Some current ->
    let pages =
      1
      :: t.total_pages
      :: List.range
           (Int.max 1 (current - t.siblings))
           (Int.min t.total_pages (current + t.siblings) + 1)
      |> List.dedup_and_sort ~compare:Int.compare
    in
    let _, reversed =
      List.fold pages ~init:(0, []) ~f:(fun (previous, reversed) page ->
        let reversed =
          match page - previous with
          | 1 -> reversed
          | 2 -> Item.Page (previous + 1) :: reversed
          | _ -> Item.Gap { first = previous + 1; last = page - 1 } :: reversed
        in
        page, Item.Page page :: reversed)
    in
    List.rev reversed
;;

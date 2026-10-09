open Core
open Gpuio

type t =
  { rows : (int, string, Int.comparator_witness) List_collection.t
  ; first : int
  ; last : int
  ; streamed_lines : int
  }

module Action = struct
  type t =
    | Append
    | Prepend
    | Stream_latest
    | Reset_latest
end

let initial_count = 1000
let extra_limit = 32
let ok = Or_error.ok_exn

let text id =
  sprintf
    "Entry %04d\n%s"
    id
    (String.concat
       ~sep:"\n"
       (List.init (1 + (Int.abs id % 3)) ~f:(fun _ -> "An idea with room to develop.")))
;;

let initial =
  { rows =
      List_collection.of_alist
        (module Int)
        (List.init initial_count ~f:(fun id -> id, text id))
      |> ok
  ; first = 0
  ; last = initial_count - 1
  ; streamed_lines = 0
  }
;;

let rows t = t.rows
let first t = t.first
let last t = t.last
let streamed_lines t = t.streamed_lines
let can_append t = t.last < initial_count + extra_limit - 1
let can_prepend t = t.first > -extra_limit

let with_streamed_lines t streamed_lines =
  if streamed_lines = t.streamed_lines
  then t
  else (
    let data =
      text t.last
      ^ String.concat (List.init streamed_lines ~f:(fun _ -> "\nAnother useful detail."))
    in
    { t with rows = List_collection.set t.rows ~key:t.last ~data |> ok; streamed_lines })
;;

let apply t = function
  | Action.Append ->
    if can_append t
    then (
      let last = t.last + 1 in
      let rows =
        List_collection.splice
          t.rows
          ~at:(List_collection.length t.rows)
          ~remove:0
          [ last, text last ]
        |> ok
      in
      { t with rows; last; streamed_lines = 0 })
    else t
  | Prepend ->
    if can_prepend t
    then (
      let first = t.first - 1 in
      let rows =
        List_collection.splice t.rows ~at:0 ~remove:0 [ first, text first ] |> ok
      in
      { t with rows; first })
    else t
  | Stream_latest -> with_streamed_lines t (Int.min 8 (t.streamed_lines + 1))
  | Reset_latest -> with_streamed_lines t 0
;;

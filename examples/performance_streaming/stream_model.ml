open Core
module C = Gpuio.List_collection

module Row = struct
  type t =
    { stream : int
    ; block : int
    ; text : string
    }
end

module Progress = struct
  type t =
    { updates : int
    ; bytes : int
    }
  [@@deriving sexp_of]
end

type t =
  { rows : (int, Row.t, Int.comparator_witness) C.t
  ; updates : int array
  }

let stream_count = 4
let fragment_bytes = 128
let fragments_per_block = 16
let create () = { rows = C.empty (module Int); updates = Array.create ~len:4 0 }
let rows t = t.rows

let progress t =
  Array.to_list t.updates
  |> List.map ~f:(fun updates -> { Progress.updates; bytes = updates * fragment_bytes })
;;

let fragment ~stream ~sequence =
  let prefix = sprintf "Stream %d / fragment %06d · λ 世界 " stream sequence in
  prefix ^ String.make (fragment_bytes - String.length prefix - 1) 'x' ^ "\n"
;;

let append t ~stream ~sequence =
  if stream < 0 || stream >= stream_count
  then Or_error.error_string "stream outside [0,4)"
  else if sequence <> t.updates.(stream) + 1 || sequence > 1_000_000
  then Or_error.error_string "stream fragment is missing, repeated or out of order"
  else
    let open Or_error.Let_syntax in
    let block = (sequence - 1) / fragments_per_block in
    let key = (block * stream_count) + stream in
    let text = fragment ~stream ~sequence in
    let%map rows =
      if (sequence - 1) % fragments_per_block = 0
      then
        C.splice
          t.rows
          ~at:(C.length t.rows)
          ~remove:0
          [ key, { Row.stream; block; text } ]
      else (
        let%bind previous =
          C.find t.rows key |> Or_error.of_option ~error:(Error.of_string "missing block")
        in
        C.set t.rows ~key ~data:{ previous with text = previous.text ^ text })
    in
    let updates = Array.copy t.updates in
    updates.(stream) <- sequence;
    { rows; updates }
;;

let validate t ~updates_per_stream =
  let open Or_error.Let_syntax in
  let%bind () =
    if
      updates_per_stream < 0
      || Array.exists t.updates ~f:(fun actual -> actual <> updates_per_stream)
    then Or_error.error_string "incomplete stream counts"
    else Ok ()
  in
  let blocks = (updates_per_stream + fragments_per_block - 1) / fragments_per_block in
  if C.length t.rows <> blocks * stream_count
  then Or_error.error_string "missing or extra history blocks"
  else
    List.fold_result (C.to_alist t.rows) ~init:() ~f:(fun () (key, row) ->
      let first = (row.block * fragments_per_block) + 1 in
      let last = Int.min updates_per_stream (first + fragments_per_block - 1) in
      let expected =
        List.init
          (last - first + 1)
          ~f:(fun offset -> fragment ~stream:row.stream ~sequence:(first + offset))
        |> String.concat
      in
      if
        key <> (row.block * stream_count) + row.stream
        || not (String.equal row.text expected)
      then Or_error.error_string "qualification model retained incorrect canonical source"
      else Ok ())
;;

let%expect_test "interleaved streams preserve memberships and reject missing fragments" =
  let ok = Or_error.ok_exn in
  let t = create () |> fun t -> append t ~stream:2 ~sequence:1 |> ok in
  let membership = C.item_ref t.rows 2 |> Option.value_exn in
  let t = append t ~stream:2 ~sequence:2 |> ok in
  print_s
    [%sexp
      (C.contains_ref t.rows membership : bool)
    , (String.length (C.find t.rows 2 |> Option.value_exn).text : int)
    , (Result.is_error (append t ~stream:2 ~sequence:2) : bool)
    , (Result.is_error (append t ~stream:1 ~sequence:2) : bool)
    , (Result.is_error (append t ~stream:4 ~sequence:1) : bool)];
  let t = ref (create ()) in
  for sequence = 1 to 19 do
    List.iter [ 3; 1; 0; 2 ] ~f:(fun stream -> t := append !t ~stream ~sequence |> ok)
  done;
  validate !t ~updates_per_stream:19 |> ok;
  print_s [%sexp (C.length !t.rows : int), (progress !t : Progress.t list)];
  [%expect
    {|
    (true 256 true true true)
    (8
     (((updates 19) (bytes 2432)) ((updates 19) (bytes 2432))
      ((updates 19) (bytes 2432)) ((updates 19) (bytes 2432))))
    |}]
;;

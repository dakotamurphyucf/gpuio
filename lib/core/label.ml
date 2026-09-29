open Core

let max_source_bytes = Gpuio_protocol.Text_content_wire.max_text_bytes
let max_runs = Gpuio_protocol.Text_content_wire.max_spans

let iter_scalars text ~f =
  let rec loop offset =
    if offset < String.length text
    then (
      let decoded = Stdlib.String.get_utf_8_uchar text offset in
      let end_byte = offset + Stdlib.Uchar.utf_decode_length decoded in
      f ~start_byte:offset ~end_byte (Stdlib.Uchar.utf_decode_uchar decoded);
      loop end_byte)
  in
  loop 0
;;

let iter_lower text ~f =
  iter_scalars text ~f:(fun ~start_byte ~end_byte scalar ->
    match Uucp.Case.Map.to_lower scalar with
    | `Self -> f ~start_byte ~end_byte scalar
    | `Uchars scalars -> List.iter scalars ~f:(f ~start_byte ~end_byte))
;;

let lowercase text ~limit =
  let buffer = Stdlib.Buffer.create (String.length text) in
  let within_limit = ref true in
  iter_lower text ~f:(fun ~start_byte:_ ~end_byte:_ scalar ->
    if !within_limit
    then (
      Stdlib.Buffer.add_utf_8_uchar buffer scalar;
      within_limit := Stdlib.Buffer.length buffer <= limit));
  if !within_limit
  then Ok (Stdlib.Buffer.contents buffer)
  else Or_error.error_string "lowercased label text exceeds its byte budget"
;;

module Match = struct
  type mode =
    | Prefix
    | All

  type t =
    { mode : mode
    ; lowered : string
    }

  let create mode query =
    if String.length query > 4096 || not (Stdlib.String.is_valid_utf_8 query)
    then Or_error.error_string "label match requires at most 4096 valid UTF-8 bytes"
    else Or_error.map (lowercase query ~limit:16384) ~f:(fun lowered -> { mode; lowered })
  ;;

  let prefix = create Prefix
  let all = create All
end

module Role = struct
  type t =
    | Secondary
    | Highlight
  [@@deriving equal, sexp_of]
end

module Run = struct
  type t =
    { start_byte : int
    ; end_byte : int
    ; role : Role.t
    }
  [@@deriving equal, sexp_of]
end

type t =
  { text : string
  ; runs : Run.t list
  }
[@@deriving equal, sexp_of]

let display_text t = t.text

let source primary secondary =
  let valid text =
    String.length text <= max_source_bytes && Stdlib.String.is_valid_utf_8 text
  in
  if not (valid primary && Option.for_all secondary ~f:valid)
  then Or_error.error_string "label source requires bounded valid UTF-8"
  else (
    match secondary with
    | None -> Ok (primary, None)
    | Some secondary ->
      if String.length primary > max_source_bytes - String.length secondary - 1
      then Or_error.error_string "combined label source exceeds 262144 bytes"
      else Ok (primary ^ " " ^ secondary, Some (String.length primary)))
;;

let masked_content text =
  let count = ref 0 in
  iter_scalars text ~f:(fun ~start_byte:_ ~end_byte:_ _ -> incr count);
  if !count > max_source_bytes / 3
  then Or_error.error_string "masked label output exceeds 262144 bytes"
  else
    Ok { text = String.init (3 * !count) ~f:(fun index -> "•".[index mod 3]); runs = [] }
;;

let match_ranges text (highlight : Match.t) =
  let open Or_error.Let_syntax in
  if String.is_empty highlight.lowered
  then Ok []
  else (
    let%map lowered = lowercase text ~limit:1048576 in
    let offsets = Array.create ~len:(String.length lowered) 0 in
    let next = ref 0 in
    iter_lower text ~f:(fun ~start_byte ~end_byte:_ scalar ->
      let length = Stdlib.Uchar.utf_8_byte_length scalar in
      Array.fill offsets ~pos:!next ~len:length start_byte;
      next := !next + length);
    let positions =
      match highlight.mode with
      | Prefix -> if String.is_prefix lowered ~prefix:highlight.lowered then [ 0 ] else []
      | All ->
        String.Search_pattern.create highlight.lowered
        |> String.Search_pattern.index_all ~may_overlap:true ~in_:lowered
    in
    List.fold positions ~init:[] ~f:(fun merged position ->
      let start_byte = offsets.(position) in
      let last = offsets.(position + String.length highlight.lowered - 1) in
      let end_byte =
        last + Stdlib.Uchar.utf_decode_length (Stdlib.String.get_utf_8_uchar text last)
      in
      match merged with
      | (previous_start, previous_end) :: rest when start_byte <= previous_end ->
        (previous_start, Int.max previous_end end_byte) :: rest
      | [] | _ :: _ -> (start_byte, end_byte) :: merged)
    |> List.rev)
;;

let runs ~length ~secondary_start matches =
  let add acc start_byte end_byte role =
    if start_byte < end_byte then { Run.start_byte; end_byte; role } :: acc else acc
  in
  let secondary acc start_byte end_byte =
    Option.value_map secondary_start ~default:acc ~f:(fun from ->
      add acc (Int.max from start_byte) end_byte Role.Secondary)
  in
  let end_byte, reversed =
    List.fold matches ~init:(0, []) ~f:(fun (previous_end, acc) (start_byte, end_byte) ->
      let acc = secondary acc previous_end start_byte in
      end_byte, add acc start_byte end_byte Role.Highlight)
  in
  let result = secondary reversed end_byte length |> List.rev in
  if List.length result > max_runs
  then Or_error.error_string "label formatting exceeds 4096 foreground runs"
  else Ok result
;;

let create ?secondary ?highlight ?(masked = false) primary =
  let open Or_error.Let_syntax in
  let%bind text, secondary_start = source primary secondary in
  if masked
  then masked_content text
  else (
    let%bind matches =
      match highlight with
      | None -> Ok []
      | Some highlight -> match_ranges text highlight
    in
    let%map runs = runs ~length:(String.length text) ~secondary_start matches in
    { text; runs })
;;

module Expert = struct
  let to_text_content t ~secondary ~highlight =
    let spans =
      List.map t.runs ~f:(fun run ->
        let foreground =
          match run.role with
          | Secondary -> secondary
          | Highlight -> highlight
        in
        Text_content.Span.create
          ~start_byte:run.start_byte
          ~end_byte:run.end_byte
          ~foreground
        |> Or_error.ok_exn)
    in
    Text_content.create ~spans t.text |> Or_error.ok_exn
  ;;
end

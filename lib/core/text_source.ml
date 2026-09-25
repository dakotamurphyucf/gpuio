open Core

module Status = struct
  type t =
    | Streaming
    | Complete
    | Cancelled
  [@@deriving equal, sexp_of]
end

type t =
  { source : unit ref
  ; generation : unit ref
  ; chunks : string Int.Map.t
  ; byte_length : int
  ; status : Status.t
  }

let max_bytes = 8 * 1024 * 1024
let chunk_bytes = 16 * 1024
let byte_length t = t.byte_length
let status t = t.status
let equal = phys_equal

let validate text =
  if String.length text > max_bytes
  then Or_error.error_string "document exceeds 8 MiB"
  else if not (Stdlib.String.is_valid_utf_8 text)
  then Or_error.error_string "document must be valid UTF-8"
  else Ok ()
;;

let boundary text offset =
  offset >= 0
  && offset <= String.length text
  && (offset = String.length text || Char.to_int text.[offset] land 0xc0 <> 0x80)
;;

let add_chunks chunks ~offset text =
  let length = String.length text in
  let rec loop chunks pos =
    if pos = length
    then chunks
    else (
      let last = ref (Int.min length (pos + chunk_bytes)) in
      while not (boundary text !last) do
        Int.decr last
      done;
      let data = String.sub text ~pos ~len:(!last - pos) in
      loop (Map.set chunks ~key:(offset + pos) ~data) !last)
  in
  loop chunks 0
;;

let of_string ?(status = Status.Complete) text =
  let%map.Or_error () = validate text in
  { source = ref ()
  ; generation = ref ()
  ; chunks = add_chunks Int.Map.empty ~offset:0 text
  ; byte_length = String.length text
  ; status
  }
;;

let empty_stream () = of_string ~status:Streaming "" |> Or_error.ok_exn
let to_string t = String.concat (Map.data t.chunks)

let append t text =
  let open Or_error.Let_syntax in
  let%bind () = validate text in
  if not (Status.equal t.status Streaming)
  then Or_error.error_string "cannot append to a terminal document; reset first"
  else if String.length text > max_bytes - t.byte_length
  then Or_error.error_string "document exceeds 8 MiB"
  else if String.is_empty text
  then Ok t
  else (
    let chunks =
      match Map.max_elt t.chunks with
      | Some (offset, tail) when String.length tail < chunk_bytes ->
        add_chunks (Map.remove t.chunks offset) ~offset (tail ^ text)
      | Some _ | None -> add_chunks t.chunks ~offset:t.byte_length text
    in
    Ok { t with chunks; byte_length = t.byte_length + String.length text })
;;

let replace t text =
  let%map.Or_error () = validate text in
  { t with
    chunks = add_chunks Int.Map.empty ~offset:0 text
  ; byte_length = String.length text
  }
;;

let interval t ~first ~last =
  if first < 0 || first > last || last > t.byte_length
  then Or_error.error_string "invalid document byte interval"
  else (
    let is_boundary offset =
      if offset = t.byte_length
      then true
      else (
        match Map.closest_key t.chunks `Less_or_equal_to offset with
        | None -> offset = 0
        | Some (start, text) -> boundary text (offset - start))
    in
    if is_boundary first && is_boundary last
    then Ok ()
    else Or_error.error_string "document offsets must be UTF-8 scalar boundaries")
;;

let slice t ~first ~last =
  let%map.Or_error () = interval t ~first ~last in
  if first = last
  then ""
  else (
    let start =
      Map.closest_key t.chunks `Less_or_equal_to first
      |> Option.value_map ~default:0 ~f:fst
    in
    Map.subrange t.chunks ~lower_bound:(Incl start) ~upper_bound:(Excl last)
    |> Map.to_alist
    |> List.map ~f:(fun (offset, text) ->
      let pos = Int.max 0 (first - offset) in
      let stop = Int.min (String.length text) (last - offset) in
      String.sub text ~pos ~len:(stop - pos))
    |> String.concat)
;;

let edit t ~first ~last ~text =
  let open Or_error.Let_syntax in
  let%bind () = interval t ~first ~last in
  let%bind () = validate text in
  if String.length text > max_bytes - (t.byte_length - (last - first))
  then Or_error.error_string "document exceeds 8 MiB"
  else (
    let%bind before = slice t ~first:0 ~last:first in
    let%bind after = slice t ~first:last ~last:t.byte_length in
    replace t (before ^ text ^ after))
;;

let terminate t status =
  if Status.equal t.status status
  then Ok t
  else (
    match t.status with
    | Streaming -> Ok { t with status }
    | Complete | Cancelled -> Or_error.error_string "document is already terminal")
;;

let finish t = terminate t Complete
let cancel t = terminate t Cancelled

let reset t ?(status = Status.Streaming) text =
  let%map.Or_error next = replace t text in
  { next with generation = ref (); status }
;;

module Owner = struct
  type t = unit ref

  let create () = ref ()
  let equal = phys_equal
end

module Decoder = struct
  type t = string

  let empty = ""
  let pending_bytes = String.length

  let incomplete_prefix text =
    let length = String.length text in
    if length = 0
    then true
    else (
      let lead = Char.to_int text.[0] in
      let expected =
        if lead >= 0xc2 && lead <= 0xdf
        then 2
        else if lead >= 0xe0 && lead <= 0xef
        then 3
        else if lead >= 0xf0 && lead <= 0xf4
        then 4
        else 0
      in
      length < expected
      && String.for_alli text ~f:(fun index byte ->
        if index = 0
        then true
        else (
          let byte = Char.to_int byte in
          byte >= 0x80
          && byte <= 0xbf
          && (index <> 1
              ||
              match lead with
              | 0xe0 -> byte >= 0xa0
              | 0xed -> byte <= 0x9f
              | 0xf0 -> byte >= 0x90
              | 0xf4 -> byte <= 0x8f
              | _ -> true))))
  ;;

  let feed t bytes =
    if String.length bytes > 262_144
    then Or_error.error_string "UTF-8 input chunk exceeds 256 KiB"
    else (
      let bytes = t ^ bytes in
      let rec find pending =
        if pending > 3 || pending > String.length bytes
        then Or_error.error_string "invalid UTF-8 stream"
        else (
          let ready = String.prefix bytes (String.length bytes - pending) in
          let tail = String.suffix bytes pending in
          if Stdlib.String.is_valid_utf_8 ready && incomplete_prefix tail
          then Ok (tail, ready)
          else find (pending + 1))
      in
      find 0)
  ;;

  let finish t =
    if String.is_empty t
    then Ok ()
    else Or_error.error_string "incomplete UTF-8 scalar at end of stream"
  ;;
end

module Handle = struct
  type t =
    { owner : Owner.t
    ; id : Gpuio_protocol.Resource_id.t
    }

  let equal t other =
    Owner.equal t.owner other.owner && Gpuio_protocol.Resource_id.equal t.id other.id
  ;;

  let sexp_of_t t = [%sexp { id = (t.id : Gpuio_protocol.Resource_id.t) }]
end

module Expert = struct
  module Owner = Owner

  let handle ~owner id = { Handle.owner; id }
  let belongs_to (t : Handle.t) ~owner = Owner.equal t.owner owner
  let native_id (t : Handle.t) = t.id
  let same_source t other = phys_equal t.source other.source
  let same_generation t other = phys_equal t.generation other.generation

  let changed_from t ~previous =
    if not (same_generation t previous)
    then 0
    else (
      match
        Map.symmetric_diff previous.chunks t.chunks ~data_equal:phys_equal
        |> Sequence.next
      with
      | None -> previous.byte_length
      | Some ((offset, `Left _), _) | Some ((offset, `Right _), _) -> offset
      | Some ((offset, `Unequal (before, after)), _) ->
        let common = ref 0 in
        let limit = Int.min (String.length before) (String.length after) in
        while !common < limit && Char.equal before.[!common] after.[!common] do
          Int.incr common
        done;
        while not (boundary before !common && boundary after !common) do
          Int.decr common
        done;
        offset + !common)
  ;;

  let iter_chunks t ~f = Map.iteri t.chunks ~f:(fun ~key ~data -> f ~offset:key data)
end

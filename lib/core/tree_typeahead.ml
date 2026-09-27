open Core
module W = Gpuio_protocol.Tree_input_wire

let max_bytes = 256

module Canonical = struct
  type t =
    { before : Uunf.t
    ; after : Uunf.t
    }

  let create () = { before = Uunf.create `NFD; after = Uunf.create `NFD }

  (* [emit=false] stops as soon as the requested prefix decides the match. Reset
     makes both normalizers reusable after stopping in the middle of a label. *)
  let iter t text ~emit =
    Uunf.reset t.before;
    Uunf.reset t.after;
    let rec after value =
      match Uunf.add t.after value with
      | `Await | `End -> true
      | `Uchar char -> emit char && after `Await
    in
    let rec before value =
      match Uunf.add t.before value with
      | `Await | `End -> true
      | `Uchar char ->
        let keep_going =
          match Uucp.Case.Fold.fold char with
          | `Self -> after (`Uchar char)
          | `Uchars chars -> List.for_all chars ~f:(fun char -> after (`Uchar char))
        in
        keep_going && before `Await
    in
    let rec loop offset =
      if offset = String.length text
      then before `End && after `End
      else (
        let decoded = Stdlib.String.get_utf_8_uchar text offset in
        before (`Uchar (Stdlib.Uchar.utf_decode_uchar decoded))
        && loop (offset + Stdlib.Uchar.utf_decode_length decoded))
    in
    ignore (loop 0 : bool)
  ;;

  let key text =
    if String.for_all text ~f:(fun char -> Char.to_int char < 128)
    then String.lowercase text
    else (
      let output = Stdlib.Buffer.create (String.length text) in
      iter (create ()) text ~emit:(fun char ->
        Stdlib.Buffer.add_utf_8_uchar output char;
        true);
      Stdlib.Buffer.contents output)
  ;;

  let matches t label prefix =
    let offset = ref 0 in
    let matched = ref true in
    iter t label ~emit:(fun char ->
      let expected = Stdlib.String.get_utf_8_uchar prefix !offset in
      if Stdlib.Uchar.equal char (Stdlib.Uchar.utf_decode_uchar expected)
      then offset := !offset + Stdlib.Uchar.utf_decode_length expected
      else matched := false;
      !matched && !offset < String.length prefix);
    !matched && !offset = String.length prefix
  ;;
end

let canonical = Canonical.key

module Input = struct
  type t =
    { text : string
    ; reset : bool
    ; cycle : bool
    }
  [@@deriving sexp_of]

  let create ~reset ~cycle text =
    if not (W.valid_typeahead_text text)
    then Or_error.error_string "tree typeahead requires 1..256 printable UTF-8 bytes"
    else (
      let text = canonical text in
      if String.is_empty text || String.length text > max_bytes
      then Or_error.error_string "normalized tree typeahead exceeds 256 bytes"
      else Ok { text; reset; cycle })
  ;;
end

type t = string [@@deriving sexp_of]

let empty = ""
let prefix_bytes = String.length

let matches normalizer label prefix =
  let rec ascii offset =
    if offset = String.length prefix
    then Some true
    else if offset = String.length label
    then Some false
    else (
      let label_char = label.[offset] in
      let prefix_char = prefix.[offset] in
      if Char.to_int label_char >= 128
      then None
      else if Char.to_int prefix_char >= 128
      then Some false
      else if Char.equal (Char.lowercase label_char) prefix_char
      then ascii (offset + 1)
      else Some false)
  in
  match ascii 0 with
  | Some matched -> matched
  | None -> Canonical.matches (Lazy.force normalizer) label prefix
;;

let find tree ~visible ~active ~include_active prefix =
  let normalizer = lazy (Canonical.create ()) in
  let rec loop visible started wrapped =
    match visible with
    | [] -> wrapped
    | id :: rest ->
      let at_active =
        match active with
        | None -> false
        | Some active -> Tree.Id.equal active id
      in
      let starts_here = started || (at_active && include_active) in
      let candidate =
        (starts_here || Option.is_none wrapped)
        &&
        match Tree.find tree id with
        | None -> false
        | Some node ->
          (not (Tree.Node.is_disabled node))
          && matches normalizer (Tree.Node.label node) prefix
      in
      if starts_here && candidate
      then Some id
      else (
        let wrapped = if candidate && Option.is_none wrapped then Some id else wrapped in
        loop rest (started || at_active) wrapped)
  in
  loop visible (Option.is_none active) None
;;

let advance t tree ~visible ~active input =
  let previous = if input.Input.reset then empty else t in
  let cycling = input.cycle && String.equal previous input.text in
  let extending = (not (String.is_empty previous)) && not cycling in
  let prefix, extending =
    if cycling
    then previous, false
    else if String.length previous + String.length input.text > max_bytes
    then input.text, false
    else canonical (previous ^ input.text), extending
  in
  (* Canonical ordering across a keystroke boundary can change combining order.
     Folding an already canonical prefix is idempotent. *)
  let prefix, extending =
    if String.length prefix > max_bytes then input.text, false else prefix, extending
  in
  match find tree ~visible ~active ~include_active:extending prefix with
  | Some id -> prefix, Some id
  | None when extending ->
    input.text, find tree ~visible ~active ~include_active:false input.text
  | None -> prefix, None
;;

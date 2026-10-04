open Core

let max_text_bytes = 262_144

let scalars text =
  let rec loop offset acc =
    if offset = String.length text
    then List.rev acc
    else (
      let decoded = Stdlib.String.get_utf_8_uchar text offset in
      loop
        (offset + Stdlib.Uchar.utf_decode_length decoded)
        (Stdlib.Uchar.to_int (Stdlib.Uchar.utf_decode_uchar decoded) :: acc))
  in
  loop 0 []
;;

let add_scalar buffer scalar =
  Stdlib.Buffer.add_utf_8_uchar buffer (Stdlib.Uchar.of_int scalar)
;;

let control scalar = scalar < 32 || (scalar >= 127 && scalar <= 159)
let digit scalar = scalar >= Char.to_int '0' && scalar <= Char.to_int '9'

let letter scalar =
  (scalar >= Char.to_int 'A' && scalar <= Char.to_int 'Z')
  || (scalar >= Char.to_int 'a' && scalar <= Char.to_int 'z')
;;

let normalize_number_scalar = function
  | scalar when scalar >= 0xff10 && scalar <= 0xff19 -> scalar - 0xff10 + 48
  | 0xff0b -> 43
  | 0xff0d | 0x2212 -> 45
  | 0xff0e | 0x3002 -> 46
  | 0xff0c -> 44
  | scalar -> scalar
;;

module Pattern = struct
  type t =
    { source : string
    ; tokens : int list
    }
  [@@deriving equal, sexp_of]

  let create source =
    if String.is_empty source || String.length source > 1024
    then Or_error.error_string "format pattern must contain 1..1024 UTF-8 bytes"
    else if not (Stdlib.String.is_valid_utf_8 source)
    then Or_error.error_string "format pattern must be UTF-8"
    else (
      let tokens = scalars source in
      if List.length tokens > 256 || List.exists tokens ~f:control
      then
        Or_error.error_string
          "format pattern requires at most 256 scalars without controls"
      else Ok { source; tokens })
  ;;

  let source t = t.source
end

module Number = struct
  type t =
    { separator : string option
    ; fraction_digits : int option
    }
  [@@deriving equal, sexp_of]

  let valid_separator text =
    if String.length text > 4 || not (Stdlib.String.is_valid_utf_8 text)
    then false
    else (
      match scalars text with
      | [ scalar ] ->
        (not
           (control scalar
            || (match Uucp.Gc.general_category (Stdlib.Uchar.of_int scalar) with
                | `Nd -> true
                | _ -> false)
            || scalar = Char.to_int '+'
            || scalar = Char.to_int '-'
            || scalar = Char.to_int '.'))
        && normalize_number_scalar scalar = scalar
      | _ -> false)
  ;;

  let create ?separator ?fraction_digits () =
    if Option.exists separator ~f:(fun s -> not (valid_separator s))
    then
      Or_error.error_string
        "number separator must be one noncontrol scalar distinct from digits, signs and \
         decimal syntax"
    else if Option.exists fraction_digits ~f:(fun n -> n < 0 || n > max_text_bytes)
    then Or_error.error_string "fraction digit limit must be in 0..262144"
    else Ok { separator; fraction_digits }
  ;;

  let separator t = t.separator
  let fraction_digits t = t.fraction_digits
end

module Error = struct
  type t =
    | Invalid_text
    | Does_not_fit
    | Limit_exceeded
  [@@deriving equal, sexp_of]
end

type t =
  | Pattern of Pattern.t
  | Number of Number.t
[@@deriving equal, sexp_of]

let pattern t = Pattern t
let number t = Number t

let validate_text text =
  if String.length text > max_text_bytes
  then Error Error.Limit_exceeded
  else if
    (not (Stdlib.String.is_valid_utf_8 text))
    || String.exists text ~f:(fun c ->
      Char.equal c '\000' || Char.equal c '\r' || Char.equal c '\n')
  then Error Error.Invalid_text
  else Ok ()
;;

let slot_accepts token scalar =
  match token with
  | token when token = Char.to_int '9' -> Some (digit scalar)
  | token when token = Char.to_int 'A' -> Some (letter scalar)
  | token when token = Char.to_int '#' -> Some (digit scalar || letter scalar)
  | token when token = Char.to_int '*' -> Some true
  | _ -> None
;;

let next_scalar text offset =
  let decoded = Stdlib.String.get_utf_8_uchar text offset in
  ( Stdlib.Uchar.to_int (Stdlib.Uchar.utf_decode_uchar decoded)
  , offset + Stdlib.Uchar.utf_decode_length decoded )
;;

let format_pattern t raw =
  let output = Buffer.create 64 in
  let rec loop tokens offset =
    if offset = String.length raw
    then Ok (Buffer.contents output)
    else (
      match tokens with
      | [] -> Error Error.Does_not_fit
      | token :: tokens ->
        let scalar, next = next_scalar raw offset in
        (match slot_accepts token scalar with
         | Some false -> Error Error.Does_not_fit
         | Some true ->
           add_scalar output scalar;
           loop tokens next
         | None ->
           add_scalar output token;
           loop tokens offset))
  in
  loop t.Pattern.tokens 0
;;

let extract_pattern t formatted =
  let output = Buffer.create 64 in
  let rec loop tokens offset =
    if offset = String.length formatted
    then Ok (Buffer.contents output)
    else (
      match tokens with
      | [] -> Error Error.Does_not_fit
      | token :: tokens ->
        let scalar, next = next_scalar formatted offset in
        (match slot_accepts token scalar with
         | Some false -> Error Error.Does_not_fit
         | Some true ->
           add_scalar output scalar;
           loop tokens next
         | None -> if token = scalar then loop tokens next else Error Error.Does_not_fit))
  in
  loop t.Pattern.tokens 0
;;

let normalize_number text =
  let output = Buffer.create (String.length text) in
  let rec loop offset =
    if offset < String.length text
    then (
      let scalar, next = next_scalar text offset in
      add_scalar output (normalize_number_scalar scalar);
      loop next)
  in
  loop 0;
  Buffer.contents output
;;

let format_number t raw =
  let text = normalize_number raw in
  let sign, unsigned =
    if String.is_prefix text ~prefix:"+" || String.is_prefix text ~prefix:"-"
    then String.prefix text 1, String.drop_prefix text 1
    else "", text
  in
  let integer, fraction =
    match String.lsplit2 unsigned ~on:'.' with
    | None -> unsigned, None
    | Some (integer, fraction) -> integer, Some fraction
  in
  let digits text = String.for_all text ~f:Char.is_digit in
  if
    (not (digits integer))
    || Option.exists fraction ~f:(fun fraction -> not (digits fraction))
  then Error Error.Does_not_fit
  else if
    Option.exists fraction ~f:(fun fraction ->
      Option.exists t.Number.fraction_digits ~f:(fun limit ->
        limit = 0 || String.length fraction > limit))
  then Error Error.Does_not_fit
  else (
    let separator = Option.value t.Number.separator ~default:"" in
    let length = String.length integer in
    let groups = Int.max 0 (length - 1) / 3 in
    let size = String.length text + (groups * String.length separator) in
    if size > max_text_bytes
    then Error Error.Limit_exceeded
    else (
      let output = Buffer.create size in
      Buffer.add_string output sign;
      String.iteri integer ~f:(fun i c ->
        if i > 0 && (length - i) mod 3 = 0 then Buffer.add_string output separator;
        Buffer.add_char output c);
      Option.iter fraction ~f:(fun fraction ->
        Buffer.add_char output '.';
        Buffer.add_string output fraction);
      Ok (Buffer.contents output)))
;;

let format_raw t raw =
  let open Result.Let_syntax in
  let%bind () = validate_text raw in
  match t with
  | Pattern pattern -> format_pattern pattern raw
  | Number number -> format_number number raw
;;

let raw_of_formatted t formatted =
  let open Result.Let_syntax in
  let%bind () = validate_text formatted in
  match t with
  | Pattern pattern -> extract_pattern pattern formatted
  | Number number ->
    let raw =
      match number.Number.separator with
      | None -> formatted
      | Some separator -> String.substr_replace_all formatted ~pattern:separator ~with_:""
    in
    let%bind canonical = format_number number raw in
    if String.equal canonical formatted then Ok raw else Error Error.Does_not_fit
;;

let accepts_formatted t text = Result.is_ok (raw_of_formatted t text)

module Expert = struct
  let to_wire = function
    | Pattern pattern -> Gpuio_protocol.Input_format_wire.Pattern (Pattern.source pattern)
    | Number number ->
      Number
        { separator = Number.separator number
        ; fraction_digits = Option.map (Number.fraction_digits number) ~f:Int64.of_int
        }
  ;;

  let of_wire = function
    | Gpuio_protocol.Input_format_wire.Pattern source ->
      Pattern.create source |> Or_error.map ~f:pattern
    | Number { separator; fraction_digits } ->
      let open Or_error.Let_syntax in
      let%bind fraction_digits =
        match fraction_digits with
        | None -> Ok None
        | Some value ->
          (match Int64.to_int value with
           | None -> Or_error.error_string "fraction digit limit overflows OCaml int"
           | Some value -> Ok (Some value))
      in
      Number.create ?separator ?fraction_digits () |> Or_error.map ~f:number
  ;;
end

open Core
type t = Rgba of int64 | Token of string [@@deriving equal, sexp_of]
let rgba ~red ~green ~blue ~alpha =
  if List.for_all [red;green;blue;alpha] ~f:(fun value -> value >= 0 && value <= 255)
  then Ok (Rgba Int64.(bit_or (shift_left (of_int red) 24) (bit_or (shift_left (of_int green) 16) (bit_or (shift_left (of_int blue) 8) (of_int alpha)))))
  else Or_error.error_string "RGBA channels must be in 0..255"
;;
let rgb_exn value =
  if value < 0 || value > 0xffffff then invalid_arg "RGB color must be in 0x000000..0xffffff";
  Rgba Int64.(bit_or (shift_left (of_int value) 8) 255L)
;;
let token name =
  if String.is_empty name || String.length name > 256 then Or_error.error_string "theme token name must contain 1..256 bytes"
  else Ok (Token name)
;;
let token_exn name = token name |> Or_error.ok_exn
module Expert = struct
  type value = t = Rgba of int64 | Token of string
  let value t = t
end

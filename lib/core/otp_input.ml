open Core
module W = Gpuio_protocol.Otp_wire
module Alphabet = W.Alphabet
module Input_error = W.Input_error

let max_input_bytes = W.max_input_bytes

module Policy = struct
  type t = W.Policy.t [@@deriving equal, sexp_of]

  let create ~length ?(alphabet = Alphabet.Digits) () =
    let t = { W.Policy.length; alphabet } in
    if W.Policy.valid t
    then Ok t
    else Or_error.error_string "OTP length must be between 1 and 32"
  ;;

  let length t = t.W.Policy.length
  let alphabet t = t.W.Policy.alphabet
end

module Value = struct
  type t = string [@@deriving equal, sexp_of]

  let empty = ""
  let to_string t = t
  let length = String.length
  let fits t ~policy = W.canonical policy t
  let is_complete t ~policy = fits t ~policy && String.length t = Policy.length policy
  let of_string policy text = W.normalize policy ~paste:false text
  let of_paste policy text = W.normalize policy ~paste:true text

  let edit t ~policy ~selection ~text ~paste =
    W.replace
      policy
      ~value:t
      ~anchor:(Text_input.Selection.anchor selection)
      ~head:(Text_input.Selection.head selection)
      ~paste
      text
    |> Result.map ~f:(fun (value, anchor, head) ->
      value, Text_input.Selection.create ~anchor ~head |> Or_error.ok_exn)
  ;;

  let replace t ~policy ~selection ~text = edit t ~policy ~selection ~text ~paste:false
  let paste t ~policy ~selection ~text = edit t ~policy ~selection ~text ~paste:true
end

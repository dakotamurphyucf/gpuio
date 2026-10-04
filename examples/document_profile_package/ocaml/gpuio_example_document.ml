open Core
module P = Gpuio.Document.Profile

module Accent = struct
  type t =
    | Indigo
    | Amber
  [@@deriving equal, sexp_of]
end

module Event = struct
  type t =
    | Inspect_code
    | Summarize_table
    | Open_badge
    | Open_card
  [@@deriving equal, sexp_of]
end

let schema =
  P.Schema.create
    ~name:"example.document"
    ~version:1
    ~fingerprint:"985f53077246b78111f091454a0d6f07c770cab3c3cb9218c114e4936f52a26d"
  |> Or_error.ok_exn
;;

let properties =
  P.Codec.create
    ~max_bytes:1
    ~encode:(function
      | Accent.Indigo -> Ok "\000"
      | Amber -> Ok "\001")
    ~decode:(function
      | "\000" -> Ok Accent.Indigo
      | "\001" -> Ok Amber
      | _ -> Or_error.error_string "invalid document accent")
  |> Or_error.ok_exn
;;

let events =
  P.Codec.create
    ~max_bytes:1
    ~encode:(function
      | Event.Inspect_code -> Ok "\001"
      | Summarize_table -> Ok "\002"
      | Open_badge -> Ok "\003"
      | Open_card -> Ok "\004")
    ~decode:(function
      | "\001" -> Ok Event.Inspect_code
      | "\002" -> Ok Summarize_table
      | "\003" -> Ok Open_badge
      | "\004" -> Ok Open_card
      | _ -> Or_error.error_string "invalid document action")
  |> Or_error.ok_exn
;;

let definition = P.Definition.create ~schema ~properties ~events |> Or_error.ok_exn
let instance ~accent ~generation = P.Instance.create definition ~generation accent

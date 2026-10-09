open Core
module W = Gpuio_protocol.Input_validation_wire

module Regex = struct
  module Matching = struct
    type t = W.Matching.t =
      | Whole_value
      | Substring
    [@@deriving equal, sexp_of]
  end

  module Source = struct
    type t = W.Source.t [@@deriving equal, sexp_of]

    let create ?(matching = Matching.Whole_value) ?(case_sensitive = true) pattern =
      let t : t = { pattern; matching; case_sensitive } in
      if W.Source.is_valid t
      then Ok t
      else
        Or_error.error_string
          "regex source must contain at most 2048 UTF-8 bytes without NUL"
    ;;

    let pattern t = t.W.Source.pattern
    let matching t = t.W.Source.matching
    let case_sensitive t = t.W.Source.case_sensitive
  end

  type t = { source : Source.t } [@@deriving equal, sexp_of]

  let source t = t.source
end

type t =
  { regex : Regex.t
  ; allow_empty : bool
  }
[@@deriving equal, sexp_of]

let regex ?(allow_empty = true) regex = { regex; allow_empty }

module Error = struct
  type t =
    | Invalid_source
    | Invalid_regex of string
    | Too_complex
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let to_wire t : W.Rule.t = { regex = Regex.source t.regex; allow_empty = t.allow_empty }
  let source_to_wire (source : Regex.Source.t) = source
  let checked_source source = { Regex.source }

  let error_of_wire : W.Error.t -> Error.t = function
    | Invalid_source -> Invalid_source
    | Invalid_regex message -> Invalid_regex message
    | Too_complex -> Too_complex
  ;;
end

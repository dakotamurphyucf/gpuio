open Core

type t =
  { lower : int
  ; upper : int
  }
[@@deriving equal, sexp_of]

let create ~lower ~upper =
  if lower < 0 || upper > 100 || lower > upper
  then Or_error.error_string "Score bounds must be ordered percentages from 0 to 100"
  else Ok { lower; upper }
;;

let all = { lower = 0; upper = 100 }
let lower t = t.lower
let upper t = t.upper
let contains t score = t.lower <= score && score <= t.upper
let describe t = sprintf "%d–%d%%" t.lower t.upper

open Core

module Entry = struct
  type t =
    { slice : int64
    ; text : string option
    ; line_color : int64 option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid_text text =
    String.length text <= 256
    && Stdlib.String.is_valid_utf_8 text
    && not (String.exists text ~f:(fun c -> Char.to_int c < 32 || Char.equal c '\127'))
  ;;

  let valid t =
    Int64.(t.slice > 0L)
    && Option.for_all t.text ~f:valid_text
    && Option.for_all t.line_color ~f:(fun n -> Int64.(n >= 0L && n <= 0xffff_ffffL))
  ;;
end

type t = Entry.t list [@@deriving bin_io, equal, sexp_of]

let valid t =
  List.length t <= 256
  && List.for_all t ~f:Entry.valid
  && (not
        (List.contains_dup
           (List.map t ~f:(fun e -> e.Entry.slice))
           ~compare:Int64.compare))
  && List.sum
       (module Int)
       t
       ~f:(fun e -> Option.value_map e.Entry.text ~default:0 ~f:String.length)
     <= 32768
;;

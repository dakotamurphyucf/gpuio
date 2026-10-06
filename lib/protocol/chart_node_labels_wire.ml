open Core

module Line = struct
  type t =
    { text : string
    ; color : int64 option
    ; font_size : float option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    String.length t.text <= 256
    && Stdlib.String.is_valid_utf_8 t.text
    && (not
          (String.exists t.text ~f:(fun c -> Char.to_int c < 32 || Char.equal c '\127')))
    && Option.for_all t.color ~f:(fun c -> Int64.(c >= 0L && c <= 0xffff_ffffL))
    && Option.for_all t.font_size ~f:(fun n ->
      Float.is_finite n && Float.(n >= 8. && n <= 32.))
  ;;
end

module Node = struct
  type t =
    { node : int64
    ; lines : Line.t list
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.node > 0L) && List.length t.lines <= 4 && List.for_all t.lines ~f:Line.valid
  ;;
end

type t = Node.t list [@@deriving bin_io, equal, sexp_of]

let valid t =
  List.length t <= 128
  && List.for_all t ~f:Node.valid
  && (not
        (List.contains_dup (List.map t ~f:(fun n -> n.Node.node)) ~compare:Int64.compare))
  && List.sum
       (module Int)
       t
       ~f:(fun node ->
         List.sum
           (module Int)
           node.Node.lines
           ~f:(fun line -> String.length line.Line.text))
     <= 32768
;;

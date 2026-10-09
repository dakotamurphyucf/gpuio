open Core

module Presentation = struct
  type t =
    | Modal
    | Embedded
  [@@deriving bin_io, equal, sexp_of]
end

module Search = struct
  type t =
    | All_terms
    | Substring
    | Unfiltered
    | External
  [@@deriving bin_io, equal, sexp_of]
end

module Escape = struct
  type t =
    | Dismiss
    | Clear_query_first
  [@@deriving bin_io, equal, sexp_of]
end

module Keywords = struct
  type t =
    { command : string
    ; words : string list
    }
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { search : Search.t
  ; searchable : bool
  ; escape : Escape.t
  ; keywords : Keywords.t list
  ; presentation : Presentation.t
  }
[@@deriving bin_io, equal, sexp_of]

let default =
  { search = All_terms
  ; searchable = true
  ; escape = Dismiss
  ; keywords = []
  ; presentation = Modal
  }
;;

let text_bytes t =
  List.sum
    (module Int)
    t.keywords
    ~f:(fun entry ->
      String.length entry.command + List.sum (module Int) entry.words ~f:String.length)
;;

let valid t =
  let text limit value =
    String.length value <= limit
    && (not (String.is_empty (String.strip value)))
    && Stdlib.String.is_valid_utf_8 value
    && not (String.contains value '\000')
  in
  List.length t.keywords <= 1024
  && List.for_all t.keywords ~f:(fun entry ->
    text 256 entry.command
    && List.length entry.words <= 64
    && List.for_all entry.words ~f:(text 4096))
  && Set.length (String.Set.of_list (List.map t.keywords ~f:(fun e -> e.command)))
     = List.length t.keywords
  && text_bytes t <= 262_144
;;

open Core

type op =
  | Upsert of int64 * int64 * string * int64 option
  | Children of int64 * int64 list
  | Remove of int64
  | Root of int64
  | Edit of int64 * int64 * string
[@@deriving bin_io]

type batch =
  { version : int
  ; base : int64
  ; next : int64
  ; ops : op list
  }
[@@deriving bin_io]

type event =
  | Ready
  | Applied of int64 * int64 * int64
  | Click of int64 * int64 * int64
  | Text of int64 * int64 * string * bool
  | Closed
  | Error of string
  | Probe of string
  | Frame of int64
[@@deriving bin_io, sexp_of]

type events = event list [@@deriving bin_io]

let encode batch =
  Bin_prot.Utils.bin_dump bin_writer_batch batch |> Bigstring.to_string |> Bytes.of_string
;;

let decode bytes =
  let buffer = Bigstring.of_string (Bytes.to_string bytes) in
  let pos_ref = ref 0 in
  let events = bin_read_events buffer ~pos_ref in
  assert (!pos_ref = Bigstring.length buffer);
  events
;;

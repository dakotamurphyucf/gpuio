open Core

type t =
  { commands : string list
  ; layout : Palette_layout_wire.t option
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  let text limit value =
    String.length value <= limit
    && (not (String.is_empty (String.strip value)))
    && Stdlib.String.is_valid_utf_8 value
    && not (String.contains value '\000')
  in
  let layout_valid =
    Option.for_all t.layout ~f:(fun layout ->
      let next = ref 0 in
      let index i =
        let valid = Int.equal i !next in
        Int.incr next;
        valid
      in
      let groups = Hash_set.create (module String) in
      List.length layout <= 1024
      && List.for_all layout ~f:(function
        | Palette_layout_wire.Entry.Command i -> index i
        | Separator -> true
        | Group (id, label, commands) ->
          text 256 id
          && (not (Hash_set.mem groups id))
          && (Hash_set.add groups id;
              true)
          && Option.for_all label ~f:(text 4096)
          && List.for_all commands ~f:index)
      && Int.equal !next (List.length t.commands))
  in
  List.length t.commands <= 1024
  && List.for_all t.commands ~f:(text 256)
  && Set.length (String.Set.of_list t.commands) = List.length t.commands
  && layout_valid
  && 7
     + List.sum (module Int) t.commands ~f:String.length
     + Option.value_map t.layout ~default:0 ~f:Palette_layout_wire.text_bytes
     <= 262_144
;;

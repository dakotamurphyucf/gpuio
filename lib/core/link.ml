open Core
module Wire = Gpuio_protocol.Link_wire

module Config = struct
  type t = Wire.t [@@deriving equal, sexp_of]

  let of_wire t =
    if Wire.valid t
    then Ok t
    else
      Or_error.error_string
        "link requires a nonblank UTF-8 label without NUL, <=4096 bytes, and tab index \
         in -1000000..1000000"
  ;;

  let create
        ~label
        ?(disabled = false)
        ?(loading = false)
        ?(tab_stop = true)
        ?(tab_index = 0)
        ()
    =
    of_wire
      { Wire.label; disabled; loading; tab_stop; tab_index = Int64.of_int tab_index }
  ;;

  let label (t : t) = t.label
  let is_disabled (t : t) = t.disabled
  let is_loading (t : t) = t.loading
  let tab_stop (t : t) = t.tab_stop
  let tab_index (t : t) = Int64.to_int_exn t.tab_index
end

module Expert = struct
  let to_wire (t : Config.t) = t
  let of_wire = Config.of_wire
end

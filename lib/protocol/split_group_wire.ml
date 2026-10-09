open Core

let max_panels = 64

let valid_text text ~max_bytes ~blank =
  String.length text <= max_bytes
  && (not (String.is_empty (if blank then String.strip text else text)))
  && Stdlib.String.is_valid_utf_8 text
  && not (String.contains text '\000')
;;

let valid_id id = valid_text id ~max_bytes:256 ~blank:false
let valid_size v = Float.is_finite v && Float.(v >= 0. && v <= 16384.)

module Panel = struct
  type t =
    { id : string
    ; label : string
    ; initial_size : float option
    ; minimum_size : float
    ; maximum_size : float
    ; visible : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_id t.id
    && valid_text t.label ~max_bytes:1024 ~blank:true
    && valid_size t.minimum_size
    && valid_size t.maximum_size
    && Float.(t.minimum_size <= t.maximum_size)
    && Option.for_all t.initial_size ~f:(fun v ->
      valid_size v && Float.(v >= t.minimum_size && v <= t.maximum_size))
  ;;
end

module Resize_request = struct
  type t =
    { id : string
    ; size : float
    ; serial : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = valid_id t.id && valid_size t.size && Int64.(t.serial > 0L)
end

module Config = struct
  type t =
    { label : string
    ; axis : Split_wire.Axis.t
    ; keyboard_step : float
    ; reset_generation : int64
    ; resize : Resize_request.t option
    ; panels : Panel.t list
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_text t.label ~max_bytes:4096 ~blank:true
    && valid_size t.keyboard_step
    && Float.(t.keyboard_step > 0.)
    && Int64.(t.reset_generation >= 0L)
    && Option.for_all t.resize ~f:Resize_request.valid
    && List.length t.panels <= max_panels
    && List.for_all t.panels ~f:Panel.valid
    && not
         (List.contains_dup
            (List.map t.panels ~f:(fun p -> p.Panel.id))
            ~compare:String.compare)
  ;;
end

module Source = struct
  type t =
    | Pointer
    | Keyboard
    | Accessibility
    | Request of int64
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Pointer | Keyboard | Accessibility -> true
    | Request serial -> Int64.(serial > 0L)
  ;;
end

module Snapshot = struct
  type t =
    { source : Source.t
    ; sizes : (string * float) list
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Source.valid t.source
    && List.length t.sizes <= max_panels
    && List.for_all t.sizes ~f:(fun (id, size) -> valid_id id && valid_size size)
    && not (List.contains_dup (List.map t.sizes ~f:fst) ~compare:String.compare)
  ;;

  let valid_for t (config : Config.t) =
    valid t
    && (match t.source with
        | Pointer | Keyboard | Accessibility -> true
        | Request serial ->
          Option.exists config.resize ~f:(fun request ->
            Int64.equal request.serial serial))
    && List.equal
         String.equal
         (List.map t.sizes ~f:fst)
         (List.map config.panels ~f:(fun p -> p.Panel.id))
    && List.for_all2_exn t.sizes config.panels ~f:(fun (_, size) p ->
      Float.(size >= p.minimum_size && size <= p.maximum_size))
  ;;
end

open Core

module Loop = struct
  type t =
    | Finite
    | Jump
    | Continuous
  [@@deriving bin_io, equal, sexp_of]
end

module Stops = struct
  type t =
    { canonical : int64 list
    ; looping : Loop.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    List.length t.canonical <= 128
    && (let rec loop previous index = function
          | [] -> true
          | head :: tail ->
            (Int64.equal head previous || Int64.equal head index)
            && loop head (Int64.succ index) tail
        in
        loop 0L 0L t.canonical)
    &&
    match t.looping with
    | Finite -> true
    | Jump -> List.length t.canonical > 1
    | Continuous ->
      List.length t.canonical > 1
      && List.exists t.canonical ~f:(fun index -> not (Int64.equal index 0L))
  ;;
end

module Layout = struct
  type t =
    { lineage : int64
    ; epoch : int64
    ; stops : Stops.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.lineage >= 0L && t.epoch >= 0L) && Option.for_all t.stops ~f:Stops.valid
  ;;
end

module Config = struct
  type t =
    { carousel : Carousel_wire.Config.t
    ; lineage : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Carousel_wire.Config.valid t.carousel
    && Int64.(t.lineage >= 0L && t.lineage <= t.carousel.revision)
  ;;

  let can_replace t previous =
    valid t
    && Carousel_wire.Config.can_replace t.carousel previous.carousel
    &&
    if Int64.equal t.lineage previous.lineage
    then
      List.equal String.equal t.carousel.ids previous.carousel.ids
      && Carousel_wire.Axis.equal t.carousel.axis previous.carousel.axis
      && Bool.equal t.carousel.looping previous.carousel.looping
    else
      Int64.(
        t.lineage > previous.lineage && t.carousel.revision > previous.carousel.revision)
  ;;
end

module Proposal = struct
  type t =
    { revision : int64
    ; geometry_epoch : int64
    ; from : string
    ; target : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.revision >= 0L && t.geometry_epoch >= 0L)
    && Carousel_wire.Config.valid_id t.from
    && Carousel_wire.Config.valid_id t.target
    && not (String.equal t.from t.target)
  ;;
end

module Request = struct
  type t =
    | Previous
    | Next
    | First
    | Last
    | Select of string
    | Layout of Layout.t
    | Auto_next of Proposal.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Previous | Next | First | Last -> true
    | Select id -> Carousel_wire.Config.valid_id id
    | Layout layout -> Layout.valid layout
    | Auto_next proposal -> Proposal.valid proposal
  ;;
end

let accepts_layout (config : Config.t) (layout : Layout.t) =
  Config.valid config
  && Layout.valid layout
  && Int64.equal config.lineage layout.lineage
  && Option.for_all layout.stops ~f:(fun stops ->
    List.length stops.canonical = List.length config.carousel.ids
    && (Loop.equal stops.looping Finite || config.carousel.looping))
;;

let accepts_request (config : Config.t) request =
  Config.valid config
  && Request.valid request
  &&
  let model = config.carousel in
  let enabled = (not model.disabled) && not (List.is_empty model.ids) in
  match request with
  | Request.Layout layout -> accepts_layout config layout
  | Previous | Next | First | Last -> enabled
  | Select id -> enabled && List.mem model.ids id ~equal:String.equal
  | Auto_next proposal ->
    enabled
    && Option.is_some model.auto_advance_ms
    && Int64.equal proposal.revision model.revision
    && Option.exists model.selected ~f:(fun index ->
      Option.exists
        (List.nth model.ids (Int64.to_int_exn index))
        ~f:(String.equal proposal.from))
    && List.mem model.ids proposal.target ~equal:String.equal
;;

module Motion = struct
  type t =
    { duration_ms : int64
    ; easing : Animation_wire.Easing.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.duration_ms > 0L && t.duration_ms <= 10_000L)
    &&
    match t.easing with
    | Linear | Ease | Ease_in | Ease_out | Ease_in_out | Ease_in_out_cubic -> true
    | Steps (count, position) -> Animation_wire.Easing.valid_steps ~count ~position
    | Cubic_bezier (x1, y1, x2, y2) ->
      List.for_all [ x1; x2 ] ~f:(fun x ->
        Float.is_finite x && Float.(x >= 0. && x <= 1.))
      && List.for_all [ y1; y2 ] ~f:Float.is_finite
  ;;
end

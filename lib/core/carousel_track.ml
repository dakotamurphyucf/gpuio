open Core
module W = Gpuio_protocol.Carousel_track_wire
module Id = Carousel.Id
module Item = Carousel.Item
module Axis = Carousel.Axis
module Auto_advance = Carousel.Auto_advance

module Motion = struct
  type t = W.Motion.t option [@@deriving equal, sexp_of]

  let immediate = None
  let default = Some { W.Motion.duration_ms = 200L; easing = Ease_out }

  let create ?(easing = Animation.Easing.ease_out) duration =
    let ms = Time_ns.Span.to_ms duration in
    if (not (Float.is_finite ms)) || Float.(ms <= 0. || ms > 10_000.)
    then
      Or_error.error_string
        "carousel motion duration must be positive and at most ten seconds"
    else
      Ok
        (Some
           { W.Motion.duration_ms = Float.iround_up_exn ms |> Int64.of_int
           ; easing = Animation.Expert.easing_to_wire easing
           })
  ;;

  module Expert = struct
    let to_wire t = t
  end
end

module Source = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; handler : Gpuio_protocol.Handler_id.t
    }
  [@@deriving equal, sexp_of]
end

module Layout = struct
  type t =
    { source : Source.t
    ; wire : W.Layout.t
    }
  [@@deriving equal, sexp_of]
end

module Proposal = struct
  type t =
    { source : Source.t
    ; wire : W.Proposal.t
    }
  [@@deriving equal, sexp_of]
end

module Request = struct
  type t =
    | Previous
    | Next
    | First
    | Last
    | Select of Id.t
    | Layout of Layout.t
    | Auto_next of Proposal.t
  [@@deriving equal, sexp_of]

  let previous = Previous
  let next = Next
  let first = First
  let last = Last
  let select id = Select id
end

type 'a t =
  { carousel : 'a Carousel.t
  ; axis : Axis.t
  ; lineage : int64
  ; layout : Layout.t option
  ; direction : Gpuio_protocol.Carousel_wire.Direction.t
  }

let max_items = Carousel.max_items
let max_metadata_bytes = Carousel.max_metadata_bytes
let items t = Carousel.items t.carousel
let selected t = Carousel.selected t.carousel
let find t id = Carousel.find t.carousel id
let axis t = t.axis
let is_looping t = Carousel.is_looping t.carousel
let is_disabled t = Carousel.is_disabled t.carousel
let auto_advance t = Carousel.auto_advance t.carousel
let stops t = Option.bind t.layout ~f:(fun layout -> layout.wire.stops)
let has_layout t = Option.is_some (stops t)
let base_wire t = Carousel.Expert.to_wire t.carousel ~axis:t.axis

let to_wire t : W.Config.t =
  { carousel = { (base_wire t) with direction = t.direction }; lineage = t.lineage }
;;

let create ?selected ?(axis = Axis.Horizontal) ?looping ?disabled ?auto_advance items =
  let%map.Or_error carousel =
    Carousel.create ?selected ?looping ?disabled ?auto_advance items
  in
  { carousel; axis; lineage = 0L; layout = None; direction = Direct }
;;

let invalidate_layout t =
  if Int64.equal t.lineage Int64.max_value
  then Or_error.error_string "carousel track collection lineage exhausted"
  else Ok { t with lineage = Int64.succ t.lineage; layout = None; direction = Direct }
;;

let with_items t values =
  let old_ids = List.map (items t) ~f:Item.id in
  let new_ids = List.map values ~f:Item.id in
  let%bind.Or_error carousel = Carousel.with_items t.carousel values in
  let t = { t with carousel } in
  if List.equal Id.equal old_ids new_ids then Ok t else invalidate_layout t
;;

let with_axis t axis =
  if Axis.equal t.axis axis
  then Ok t
  else (
    let%bind.Or_error carousel = Carousel.restart_auto_advance t.carousel in
    invalidate_layout { t with carousel; axis })
;;

let with_looping t looping =
  if Bool.equal (is_looping t) looping
  then Ok t
  else (
    let%bind.Or_error carousel = Carousel.with_looping t.carousel looping in
    invalidate_layout { t with carousel })
;;

let with_disabled t disabled =
  let%map.Or_error carousel = Carousel.with_disabled t.carousel disabled in
  { t with carousel }
;;

let with_auto_advance t auto_advance =
  let%map.Or_error carousel = Carousel.with_auto_advance t.carousel auto_advance in
  { t with carousel }
;;

let restart_auto_advance t =
  let%map.Or_error carousel = Carousel.restart_auto_advance t.carousel in
  { t with carousel }
;;

let select_with_direction t direction id =
  let%map.Or_error carousel = Carousel.select t.carousel id in
  if Option.exists (selected t) ~f:(fun current -> Id.equal id (Item.id current))
  then { t with carousel }
  else { t with carousel; direction }
;;

let select t id = select_with_direction t Direct id

let step t ~forward =
  if is_disabled t
  then None
  else (
    let%bind.Option stops = stops t in
    let%bind.Option current = (base_wire t).selected |> Option.map ~f:Int64.to_int_exn in
    let canonical = List.to_array stops.canonical in
    let count = Array.length canonical in
    let rec search distance =
      if
        distance >= count
        || (W.Loop.equal stops.looping Finite
            && if forward then current + distance >= count else distance > current)
      then None
      else (
        let next = (current + if forward then distance else count - distance) % count in
        if Int64.equal canonical.(next) canonical.(current)
        then search (distance + 1)
        else List.nth (items t) next)
    in
    search 1)
;;

let can_previous t = Option.is_some (step t ~forward:false)
let can_next t = Option.is_some (step t ~forward:true)

let observe t (layout : Layout.t) =
  if not (W.accepts_layout (to_wire t) layout.wire)
  then t
  else (
    match t.layout with
    | Some previous
      when Source.equal previous.source layout.source
           && Int64.(layout.wire.epoch <= previous.wire.epoch) -> t
    | None | Some _ -> { t with layout = Some layout })
;;

let apply_request t (request : Request.t) =
  let move direction item =
    match item with
    | None -> Ok t
    | Some item -> select_with_direction t direction (Item.id item)
  in
  match request with
  | Layout layout -> Ok (observe t layout)
  | _ when is_disabled t || List.is_empty (items t) -> Ok t
  | Previous -> move Previous (step t ~forward:false)
  | Next -> move Next (step t ~forward:true)
  | First -> move Direct (List.hd (items t))
  | Last -> move Direct (List.last (items t))
  | Select id -> move Direct (find t id)
  | Auto_next proposal ->
    let current = selected t in
    let next = step t ~forward:true in
    let eligible =
      Option.is_some (auto_advance t)
      && Int64.equal proposal.wire.revision (base_wire t).revision
      && Option.exists t.layout ~f:(fun layout ->
        Source.equal layout.source proposal.source
        && Int64.equal layout.wire.epoch proposal.wire.geometry_epoch)
      && Option.exists current ~f:(fun item ->
        String.equal proposal.wire.from (Id.to_string (Item.id item)))
      && Option.exists next ~f:(fun item ->
        String.equal proposal.wire.target (Id.to_string (Item.id item)))
    in
    if eligible then move Next next else Ok t
;;

module Expert = struct
  let to_wire = to_wire

  let request_of_wire ~window ~node ~handler (request : W.Request.t) =
    if not (W.Request.valid request)
    then Or_error.error_string "invalid carousel track request"
    else (
      let source = Source.{ window; node; handler } in
      match request with
      | Previous -> Ok Request.Previous
      | Next -> Ok Request.Next
      | First -> Ok Request.First
      | Last -> Ok Request.Last
      | Select id -> Id.of_string id |> Or_error.map ~f:Request.select
      | Layout wire -> Ok (Request.Layout { source; wire })
      | Auto_next wire -> Ok (Request.Auto_next { source; wire }))
  ;;
end

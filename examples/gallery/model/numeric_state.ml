open Core
module Rating = Gpuio.Rating

type t =
  { rating : Rating.Config.t
  ; custom_rating_colors : bool
  ; step_down_rating : bool
  }

module Action = struct
  type t =
    | Toggle_read_only
    | Cycle_rating_maximum
    | Cycle_rating_size
    | Toggle_rating_disabled
    | Toggle_rating_colors
    | Toggle_rating_step_down
    | Rate of Rating.Request.t
end

let initial =
  { rating = Rating.Config.create ~label:"Preview rating" ~value:3 () |> Or_error.ok_exn
  ; custom_rating_colors = false
  ; step_down_rating = false
  }
;;

let is_read_only t = Rating.Config.is_read_only t.rating
let rating t = t.rating
let custom_rating_colors t = t.custom_rating_colors
let step_down_rating t = t.step_down_rating

let configure t ?maximum ?star_size ?disabled ?read_only () =
  let maximum = Option.value maximum ~default:(Rating.Config.maximum t.rating) in
  let rating =
    Rating.Config.create
      ~label:"Preview rating"
      ~value:(Int.min maximum (Rating.Config.value t.rating))
      ~maximum
      ~star_size:(Option.value star_size ~default:(Rating.Config.star_size t.rating))
      ~disabled:(Option.value disabled ~default:(Rating.Config.is_disabled t.rating))
      ~read_only:(Option.value read_only ~default:(is_read_only t))
      ()
    |> Or_error.ok_exn
  in
  { t with rating }
;;

let apply t = function
  | Action.Toggle_read_only -> configure t ~read_only:(not (is_read_only t)) ()
  | Cycle_rating_maximum ->
    let maximum =
      match Rating.Config.maximum t.rating with
      | 5 -> 10
      | 10 -> 1
      | _ -> 5
    in
    configure t ~maximum ()
  | Cycle_rating_size ->
    let size = Rating.Config.star_size t.rating in
    let star_size =
      if Float.(size < 24.) then 24. else if Float.(size < 36.) then 36. else 16.
    in
    configure t ~star_size ()
  | Toggle_rating_disabled ->
    configure t ~disabled:(not (Rating.Config.is_disabled t.rating)) ()
  | Toggle_rating_colors -> { t with custom_rating_colors = not t.custom_rating_colors }
  | Toggle_rating_step_down -> { t with step_down_rating = not t.step_down_rating }
  | Rate request ->
    let request =
      match request with
      | Toggle index when t.step_down_rating && Rating.Config.value t.rating >= index ->
        Rating.Request.set (index - 1) |> Or_error.ok_exn
      | Set _ | Toggle _ | Increase | Decrease -> request
    in
    { t with rating = Rating.Config.apply_request t.rating request }
;;

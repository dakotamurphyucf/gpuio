open Core
module Rating = Gpuio.Rating

type t =
  | Request of Rating.Request.t
  | Toggle_read_only
  | Toggle_disabled

let initial =
  Rating.Config.create ~label:"Response quality" ~value:2 () |> Or_error.ok_exn
;;

let configure model ~read_only ~disabled =
  Rating.Config.create
    ~label:"Response quality"
    ~value:(Rating.Config.value model)
    ~read_only
    ~disabled
    ()
  |> Or_error.ok_exn
;;

let apply model = function
  | Request request -> Rating.Config.apply_request model request
  | Toggle_read_only ->
    configure
      model
      ~read_only:(not (Rating.Config.is_read_only model))
      ~disabled:(Rating.Config.is_disabled model)
  | Toggle_disabled ->
    configure
      model
      ~read_only:(Rating.Config.is_read_only model)
      ~disabled:(not (Rating.Config.is_disabled model))
;;

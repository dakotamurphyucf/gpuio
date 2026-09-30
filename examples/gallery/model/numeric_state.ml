open Core
module Rating = Gpuio.Rating

type t = Rating.Config.t

module Action = struct
  type t =
    | Toggle_read_only
    | Rate of Rating.Request.t
end

let initial = Rating.Config.create ~label:"Preview rating" ~value:3 () |> Or_error.ok_exn
let is_read_only = Rating.Config.is_read_only
let rating t = t

let apply t = function
  | Action.Toggle_read_only ->
    Rating.Config.create
      ~label:"Preview rating"
      ~value:(Rating.Config.value t)
      ~read_only:(not (is_read_only t))
      ()
    |> Or_error.ok_exn
  | Rate request -> Rating.Config.apply_request t request
;;

open Core
module Wire = Gpuio_protocol.Button_wire

module Focus = struct
  type t =
    | Focusable of Tab_order.t
    | Preserve
  [@@deriving equal, sexp_of]

  let default = Focusable Tab_order.default

  let to_wire = function
    | Focusable order -> Wire.Focus.Focusable (Tab_order.Expert.to_wire order)
    | Preserve -> Wire.Focus.Preserve
  ;;

  let of_wire = function
    | Wire.Focus.Focusable order ->
      Tab_order.Expert.of_wire order |> Or_error.map ~f:(fun order -> Focusable order)
    | Preserve -> Ok Preserve
  ;;
end

module Config = struct
  type t =
    { loading : bool
    ; focus : Focus.t
    }
  [@@deriving equal, sexp_of]

  let create ?(loading = false) ?(focus = Focus.default) () = { loading; focus }
  let default = create ()
  let is_loading t = t.loading
  let focus t = t.focus
end

module Expert = struct
  module Presentation = struct
    type t =
      { config : Config.t
      ; content : Wire.Content.t
      }
    [@@deriving equal, sexp_of]

    let icon_slots config = { config; content = Wire.Content.Icon_slots }
    let rich config = { config; content = Wire.Content.Rich }
    let config t = t.config

    let to_wire t =
      { Wire.Config.policy =
          { loading = Config.is_loading t.config
          ; focus = Focus.to_wire (Config.focus t.config)
          }
      ; content = t.content
      }
    ;;
  end

  let to_wire (t : Config.t) =
    { Wire.Policy.loading = t.loading; focus = Focus.to_wire t.focus }
  ;;

  let of_wire (t : Wire.Policy.t) =
    Focus.of_wire t.focus
    |> Or_error.map ~f:(fun focus -> Config.create ~loading:t.loading ~focus ())
  ;;
end

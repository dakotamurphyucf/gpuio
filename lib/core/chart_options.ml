open Core
module Wire = Gpuio_protocol.Chart_options_wire

let checked valid value message =
  if valid value then Ok value else Or_error.error_string message
;;

module Number_format = struct
  type t = Wire.Number_format.t [@@deriving equal, sexp_of]

  let compact = Wire.Number_format.Compact

  let precision constructor ~decimals =
    checked
      Wire.Number_format.valid
      (constructor (Int64.of_int decimals))
      "chart number format decimals must be in [0,6]"
  ;;

  let fixed = precision (fun n -> Wire.Number_format.Fixed n)
  let scientific = precision (fun n -> Wire.Number_format.Scientific n)
  let percent = precision (fun n -> Wire.Number_format.Percent n)
end

module Axes = struct
  type t = Wire.Axes.t [@@deriving equal, sexp_of]

  let create
        ?(x = true)
        ?(y = true)
        ?(grid = true)
        ?(ticks = 5)
        ?(x_format = Number_format.compact)
        ?(y_format = Number_format.compact)
        ()
    =
    checked
      Wire.Axes.valid
      { Wire.Axes.x; y; grid; ticks = Int64.of_int ticks; x_format; y_format }
      "chart axis ticks must be in [2,12]"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Curve = struct
  type t = Wire.Curve.t =
    | Linear
    | Natural
    | Step_after
  [@@deriving equal, sexp_of]
end

module Orientation = struct
  type t = Wire.Orientation.t =
    | Vertical
    | Horizontal
    | Vertical_reversed
    | Horizontal_reversed
  [@@deriving equal, sexp_of]
end

module Category_layout = struct
  type t = Wire.Category_layout.t [@@deriving equal, sexp_of]

  let auto = Wire.Category_layout.Auto

  let point ?(padding = 0.) () =
    checked
      Wire.Category_layout.valid
      (Wire.Category_layout.Point padding)
      "category point padding must be in [0,1]"
  ;;

  let band ?(inner_padding = 0.2) ?(outer_padding = 0.1) () =
    checked
      Wire.Category_layout.valid
      (Wire.Category_layout.Band { inner = inner_padding; outer = outer_padding })
      "category band inner padding must be in [0,1), outer padding in [0,1]"
  ;;
end

module Stacking = Wire.Stacking

module Cartesian = struct
  type t = Wire.Cartesian.t [@@deriving equal, sexp_of]

  let create
        ?(curve = Curve.Linear)
        ?(dots = false)
        ?(orientation = Orientation.Vertical)
        ?(bar_width = 0.8)
        ?(category_layout = Category_layout.auto)
        ?(stacking = Stacking.Grouped)
        ()
    =
    checked
      Wire.Cartesian.valid
      { Wire.Cartesian.curve; dots; orientation; bar_width; category_layout; stacking }
      "chart bar_width must be finite and in (0,1]"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Pie = struct
  type t = Wire.Pie.t [@@deriving equal, sexp_of]

  let create ?(inner_radius = 0.) ?(pad_angle = 0.) ?(labels = true) () =
    checked
      Wire.Pie.valid
      { Wire.Pie.inner_radius; pad_angle; labels }
      "chart inner_radius must be in [0,0.95] and pad_angle in [0,0.2]"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Radar = struct
  type t = Wire.Radar.t [@@deriving equal, sexp_of]

  module Scale = struct
    type t = Wire.Radar.Scale.t =
      | Per_axis
      | Data_max
      | Maximum of float
    [@@deriving equal, sexp_of]
  end

  module Radius = struct
    type t = Wire.Radar.Radius.t =
      | Fit
      | Pixels of float
    [@@deriving equal, sexp_of]
  end

  let create
        ?(levels = 4)
        ?(dots = true)
        ?(labels = true)
        ?(scale = Scale.Per_axis)
        ?(radius = Radius.Fit)
        ?(label_gap = 0.)
        ()
    =
    checked
      Wire.Radar.valid
      { Wire.Radar.levels = Int64.of_int levels; dots; labels; scale; radius; label_gap }
      "chart radar requires levels [1,12], maximum (0,1e100], radius (0,32768], gap \
       [0,64]; all floats finite"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Candlestick = struct
  type t = Wire.Candlestick.t [@@deriving equal, sexp_of]

  let create ?(body_width = 0.7) () =
    checked
      Wire.Candlestick.valid
      { Wire.Candlestick.body_width }
      "chart candle body_width must be finite and in (0,1]"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Sankey = struct
  module Alignment = struct
    type t = Wire.Sankey.Alignment.t =
      | Left
      | Right
      | Center
      | Justify
    [@@deriving equal, sexp_of]
  end

  module Scale = struct
    type t = Wire.Sankey.Scale.t =
      | Linear
      | Sqrt
    [@@deriving equal, sexp_of]
  end

  module Link_color = struct
    type t = Wire.Sankey.Link_color.t =
      | Source
      | Target
      | Gradient
    [@@deriving equal, sexp_of]
  end

  module Label_placement = struct
    type t = Wire.Sankey.Label_placement.t =
      | Inside
      | Outside
    [@@deriving equal, sexp_of]
  end

  type t = Wire.Sankey.t [@@deriving equal, sexp_of]

  let create
        ?(node_width = 16.)
        ?(node_padding = 12.)
        ?(alignment = Alignment.Justify)
        ?(scale = Scale.Linear)
        ?(iterations = 6)
        ?(labels = true)
        ?(node_corner_radius = 1.)
        ?(link_opacity = 0.5)
        ?(min_link_width = 0.)
        ?(label_gap = 6.)
        ?(link_color = Link_color.Source)
        ?(label_placement = Label_placement.Inside)
        ()
    =
    checked
      Wire.Sankey.valid
      { Wire.Sankey.node_width
      ; node_padding
      ; alignment
      ; scale
      ; iterations = Int64.of_int iterations
      ; labels
      ; node_corner_radius
      ; link_opacity
      ; min_link_width
      ; label_gap
      ; link_color
      ; label_placement
      }
      "invalid Sankey options: width [1,64], padding/gap/minimum link width [0,64], \
       iterations/radius [0,32], opacity [0,1]"
  ;;

  let default = create () |> Or_error.ok_exn
end

type t = Wire.t [@@deriving equal, sexp_of]

let create
      ?(axes = Axes.default)
      ?(cartesian = Cartesian.default)
      ?(pie = Pie.default)
      ?(radar = Radar.default)
      ?(candlestick = Candlestick.default)
      ?(sankey = Sankey.default)
      ()
  =
  { Wire.version = 7L; axes; cartesian; pie; radar; candlestick; sankey }
;;

let default = create ()

module Expert = struct
  let to_wire t = t
  let of_wire t = checked Wire.valid t "invalid chart options"
end

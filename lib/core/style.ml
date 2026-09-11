open Core
module Wire = Gpuio_protocol.Wire

module Display = struct
 type t = Block | Flex | Grid | Hidden [@@deriving equal, sexp_of]
 let to_int64 = function
 | Block -> 0L
 | Flex -> 1L
 | Grid -> 2L
 | Hidden -> 3L
end

module Visibility = struct
 type t = Visible | Hidden [@@deriving equal, sexp_of]
 let to_int64 = function
 | Visible -> 0L
 | Hidden -> 1L
end

module Direction = struct
 type t = Row | Column | Row_reverse | Column_reverse [@@deriving equal, sexp_of]
 let to_int64 = function
 | Row -> 0L
 | Column -> 1L
 | Row_reverse -> 2L
 | Column_reverse -> 3L
end

module Wrap = struct
 type t = No_wrap | Wrap | Wrap_reverse [@@deriving equal, sexp_of]
 let to_int64 = function
 | No_wrap -> 0L
 | Wrap -> 1L
 | Wrap_reverse -> 2L
end

module Align = struct
 type t = Start | End | Flex_start | Flex_end | Center | Baseline | Stretch [@@deriving equal, sexp_of]
 let to_int64 = function
 | Start -> 0L
 | End -> 1L
 | Flex_start -> 2L
 | Flex_end -> 3L
 | Center -> 4L
 | Baseline -> 5L
 | Stretch -> 6L
end

module Distribution = struct
 type t = Start | End | Flex_start | Flex_end | Center | Stretch | Space_between | Space_evenly | Space_around [@@deriving equal, sexp_of]
 let to_int64 = function
 | Start -> 0L
 | End -> 1L
 | Flex_start -> 2L
 | Flex_end -> 3L
 | Center -> 4L
 | Stretch -> 5L
 | Space_between -> 6L
 | Space_evenly -> 7L
 | Space_around -> 8L
end

module Grid_minimum = struct
 type t = Zero | Min_content | Max_content [@@deriving equal, sexp_of]
 let to_int64 = function
 | Zero -> 0L
 | Min_content -> 1L
 | Max_content -> 2L
end

module Position = struct
 type t = Relative | Absolute [@@deriving equal, sexp_of]
 let to_int64 = function
 | Relative -> 0L
 | Absolute -> 1L
end

module Text_align = struct
 type t = Left | Center | Right [@@deriving equal, sexp_of]
 let to_int64 = function
 | Left -> 0L
 | Center -> 1L
 | Right -> 2L
end

module White_space = struct
 type t = Normal | No_wrap [@@deriving equal, sexp_of]
 let to_int64 = function
 | Normal -> 0L
 | No_wrap -> 1L
end

module Text_overflow = struct
 type t = Clip | Ellipsis [@@deriving equal, sexp_of]
 let to_int64 = function
 | Clip -> 0L
 | Ellipsis -> 1L
end

module Text_decoration = struct
 type t = None | Underline | Strikethrough | Underline_and_strikethrough [@@deriving equal, sexp_of]
 let to_int64 = function
 | None -> 0L
 | Underline -> 1L
 | Strikethrough -> 2L
 | Underline_and_strikethrough -> 3L
end

module Overflow = struct
 type t = Visible | Clip | Hidden | Scroll [@@deriving equal, sexp_of]
 let to_int64 = function
 | Visible -> 0L
 | Clip -> 1L
 | Hidden -> 2L
 | Scroll -> 3L
end

module Cursor = struct
 type t = Arrow | Ibeam | Pointer | Crosshair | Move | Not_allowed | Resize_horizontal | Resize_vertical | Grab | Grabbing [@@deriving equal, sexp_of]
 let to_int64 = function
 | Arrow -> 0L
 | Ibeam -> 1L
 | Pointer -> 2L
 | Crosshair -> 3L
 | Move -> 4L
 | Not_allowed -> 5L
 | Resize_horizontal -> 6L
 | Resize_vertical -> 7L
 | Grab -> 8L
 | Grabbing -> 9L
end

module State = struct
 type t = Base | Focused | Hovered | Pressed [@@deriving equal, sexp_of]
 let to_int64 = function
 | Base -> 0L
 | Focused -> 1L
 | Hovered -> 2L
 | Pressed -> 3L
end

module Property = struct
 type t =
 | Display of Display.t
 | Visibility of Visibility.t
 | Direction of Direction.t
 | Wrap of Wrap.t
 | Grow of float
 | Shrink of float
 | Basis of Length.t
 | Align_items of Align.t
 | Align_self of Align.t
 | Align_content of Distribution.t
 | Justify_content of Distribution.t
 | Row_gap of Length.t
 | Column_gap of Length.t
 | Grid_columns of int
 | Grid_rows of int
 | Grid_column_minimum of Grid_minimum.t
 | Grid_row_minimum of Grid_minimum.t
 | Width of Length.t
 | Height of Length.t
 | Min_width of Length.t
 | Min_height of Length.t
 | Max_width of Length.t
 | Max_height of Length.t
 | Padding_top of Length.t
 | Padding_right of Length.t
 | Padding_bottom of Length.t
 | Padding_left of Length.t
 | Margin_top of Length.t
 | Margin_right of Length.t
 | Margin_bottom of Length.t
 | Margin_left of Length.t
 | Position of Position.t
 | Top of Length.t
 | Right of Length.t
 | Bottom of Length.t
 | Left of Length.t
 | Background of Background.t
 | Foreground of Color.t
 | Opacity of float
 | Border_top_width of float
 | Border_right_width of float
 | Border_bottom_width of float
 | Border_left_width of float
 | Top_left_radius of float
 | Top_right_radius of float
 | Bottom_left_radius of float
 | Bottom_right_radius of float
 | Border_color of Color.t
 | Shadows of Shadow.t list
 | Font_size of float
 | Font_family of string
 | Font_weight of int
 | Text_align of Text_align.t
 | Line_height of Length.t
 | White_space of White_space.t
 | Text_overflow of Text_overflow.t
 | Line_clamp of int
 | Text_decoration of Text_decoration.t
 | Overflow_x of Overflow.t
 | Overflow_y of Overflow.t
 | Cursor of Cursor.t
 | Pointer_events of bool
 | Padding of Length.t
 | Margin of Length.t
 | Gap of Length.t
 | Border_width of float
 | Radius of float
 | Overflow of Overflow.t
 [@@deriving equal, sexp_of]
 module Name = struct
  module T = struct
   type t =
   | Display
   | Visibility
   | Direction
   | Wrap
   | Grow
   | Shrink
   | Basis
   | Align_items
   | Align_self
   | Align_content
   | Justify_content
   | Row_gap
   | Column_gap
   | Grid_columns
   | Grid_rows
   | Grid_column_minimum
   | Grid_row_minimum
   | Width
   | Height
   | Min_width
   | Min_height
   | Max_width
   | Max_height
   | Padding_top
   | Padding_right
   | Padding_bottom
   | Padding_left
   | Margin_top
   | Margin_right
   | Margin_bottom
   | Margin_left
   | Position
   | Top
   | Right
   | Bottom
   | Left
   | Background
   | Foreground
   | Opacity
   | Border_top_width
   | Border_right_width
   | Border_bottom_width
   | Border_left_width
   | Top_left_radius
   | Top_right_radius
   | Bottom_left_radius
   | Bottom_right_radius
   | Border_color
   | Shadows
   | Font_size
   | Font_family
   | Font_weight
   | Text_align
   | Line_height
   | White_space
   | Text_overflow
   | Line_clamp
   | Text_decoration
   | Overflow_x
   | Overflow_y
   | Cursor
   | Pointer_events
   [@@deriving compare, equal, sexp]
  end
  include T
  include Comparable.Make(T)
 end

 let name = function
 | Display _ -> Name.Display
 | Visibility _ -> Name.Visibility
 | Direction _ -> Name.Direction
 | Wrap _ -> Name.Wrap
 | Grow _ -> Name.Grow
 | Shrink _ -> Name.Shrink
 | Basis _ -> Name.Basis
 | Align_items _ -> Name.Align_items
 | Align_self _ -> Name.Align_self
 | Align_content _ -> Name.Align_content
 | Justify_content _ -> Name.Justify_content
 | Row_gap _ -> Name.Row_gap
 | Column_gap _ -> Name.Column_gap
 | Grid_columns _ -> Name.Grid_columns
 | Grid_rows _ -> Name.Grid_rows
 | Grid_column_minimum _ -> Name.Grid_column_minimum
 | Grid_row_minimum _ -> Name.Grid_row_minimum
 | Width _ -> Name.Width
 | Height _ -> Name.Height
 | Min_width _ -> Name.Min_width
 | Min_height _ -> Name.Min_height
 | Max_width _ -> Name.Max_width
 | Max_height _ -> Name.Max_height
 | Padding_top _ -> Name.Padding_top
 | Padding_right _ -> Name.Padding_right
 | Padding_bottom _ -> Name.Padding_bottom
 | Padding_left _ -> Name.Padding_left
 | Margin_top _ -> Name.Margin_top
 | Margin_right _ -> Name.Margin_right
 | Margin_bottom _ -> Name.Margin_bottom
 | Margin_left _ -> Name.Margin_left
 | Position _ -> Name.Position
 | Top _ -> Name.Top
 | Right _ -> Name.Right
 | Bottom _ -> Name.Bottom
 | Left _ -> Name.Left
 | Background _ -> Name.Background
 | Foreground _ -> Name.Foreground
 | Opacity _ -> Name.Opacity
 | Border_top_width _ -> Name.Border_top_width
 | Border_right_width _ -> Name.Border_right_width
 | Border_bottom_width _ -> Name.Border_bottom_width
 | Border_left_width _ -> Name.Border_left_width
 | Top_left_radius _ -> Name.Top_left_radius
 | Top_right_radius _ -> Name.Top_right_radius
 | Bottom_left_radius _ -> Name.Bottom_left_radius
 | Bottom_right_radius _ -> Name.Bottom_right_radius
 | Border_color _ -> Name.Border_color
 | Shadows _ -> Name.Shadows
 | Font_size _ -> Name.Font_size
 | Font_family _ -> Name.Font_family
 | Font_weight _ -> Name.Font_weight
 | Text_align _ -> Name.Text_align
 | Line_height _ -> Name.Line_height
 | White_space _ -> Name.White_space
 | Text_overflow _ -> Name.Text_overflow
 | Line_clamp _ -> Name.Line_clamp
 | Text_decoration _ -> Name.Text_decoration
 | Overflow_x _ -> Name.Overflow_x
 | Overflow_y _ -> Name.Overflow_y
 | Cursor _ -> Name.Cursor
 | Pointer_events _ -> Name.Pointer_events
 | Padding _ | Margin _ | Gap _ | Border_width _ | Radius _ | Overflow _ -> assert false

 let expand = function
 | Padding v -> [Padding_top v;Padding_right v;Padding_bottom v;Padding_left v]
 | Margin v -> [Margin_top v;Margin_right v;Margin_bottom v;Margin_left v]
 | Gap v -> [Row_gap v;Column_gap v]
 | Border_width v -> [Border_top_width v;Border_right_width v;Border_bottom_width v;Border_left_width v]
 | Radius v -> [Top_left_radius v;Top_right_radius v;Bottom_left_radius v;Bottom_right_radius v]
 | Overflow v -> [Overflow_x v;Overflow_y v]
 | property -> [property]
 ;;
 let validate property =
  let nonnegative v = Float.is_finite v && Float.(v >= 0. && v <= 1_000_000.) in
  let length v ~auto ~negative = match Length.Expert.to_wire v with
    | Auto -> auto
    | Px n | Percent n -> negative || Float.(n >= 0.) in
  let valid = match property with
 | Display _ -> true
 | Visibility _ -> true
 | Direction _ -> true
 | Wrap _ -> true
 | Grow v -> nonnegative v
 | Shrink v -> nonnegative v
 | Basis v -> length v ~auto:true ~negative:false
 | Align_items _ -> true
 | Align_self _ -> true
 | Align_content _ -> true
 | Justify_content _ -> true
 | Row_gap v -> length v ~auto:false ~negative:false
 | Column_gap v -> length v ~auto:false ~negative:false
 | Grid_columns v -> v >= 1 && v <= 1024
 | Grid_rows v -> v >= 1 && v <= 1024
 | Grid_column_minimum _ -> true
 | Grid_row_minimum _ -> true
 | Width v -> length v ~auto:true ~negative:false
 | Height v -> length v ~auto:true ~negative:false
 | Min_width v -> length v ~auto:true ~negative:false
 | Min_height v -> length v ~auto:true ~negative:false
 | Max_width v -> length v ~auto:true ~negative:false
 | Max_height v -> length v ~auto:true ~negative:false
 | Padding_top v -> length v ~auto:false ~negative:false
 | Padding_right v -> length v ~auto:false ~negative:false
 | Padding_bottom v -> length v ~auto:false ~negative:false
 | Padding_left v -> length v ~auto:false ~negative:false
 | Margin_top v -> length v ~auto:true ~negative:true
 | Margin_right v -> length v ~auto:true ~negative:true
 | Margin_bottom v -> length v ~auto:true ~negative:true
 | Margin_left v -> length v ~auto:true ~negative:true
 | Position _ -> true
 | Top v -> length v ~auto:true ~negative:true
 | Right v -> length v ~auto:true ~negative:true
 | Bottom v -> length v ~auto:true ~negative:true
 | Left v -> length v ~auto:true ~negative:true
 | Background _ -> true
 | Foreground _ -> true
 | Opacity v -> Float.is_finite v && Float.(v >= 0. && v <= 1.)
 | Border_top_width v -> nonnegative v
 | Border_right_width v -> nonnegative v
 | Border_bottom_width v -> nonnegative v
 | Border_left_width v -> nonnegative v
 | Top_left_radius v -> nonnegative v
 | Top_right_radius v -> nonnegative v
 | Bottom_left_radius v -> nonnegative v
 | Bottom_right_radius v -> nonnegative v
 | Border_color _ -> true
 | Shadows v -> List.length v <= 8
 | Font_size v -> nonnegative v && Float.(v > 0.)
 | Font_family v -> not (String.is_empty v) && String.length v <= 256
 | Font_weight v -> v >= 1 && v <= 1000
 | Text_align _ -> true
 | Line_height v -> length v ~auto:false ~negative:false
 | White_space _ -> true
 | Text_overflow _ -> true
 | Line_clamp v -> v >= 1 && v <= 1024
 | Text_decoration _ -> true
 | Overflow_x _ -> true
 | Overflow_y _ -> true
 | Cursor _ -> true
 | Pointer_events _ -> true
 | Padding _ | Margin _ | Gap _ | Border_width _ | Radius _ | Overflow _ -> assert false
 in if valid then Ok () else Or_error.error_s [%message "invalid style property" (property:t)]
;;
end

type t = Property.t option Property.Name.Map.t Int.Map.t [@@deriving equal, sexp_of]
let empty = Int.Map.empty
let state_index state = State.to_int64 state |> Int64.to_int_exn
let with_state t state properties =
 if List.length properties > 128 then Or_error.error_string "too many style declarations"
 else
  let properties=List.concat_map properties ~f:Property.expand in
  let%map.Or_error properties=List.fold_result properties ~init:Property.Name.Map.empty ~f:(fun result property ->
    let%map.Or_error ()=Property.validate property in
    Map.set result ~key:(Property.name property) ~data:(Some property)) in
  Map.set t ~key:(state_index state) ~data:properties
;;
let with_state_exn t state properties = with_state t state properties |> Or_error.ok_exn
let create properties = with_state empty State.Base properties
let create_exn properties = create properties |> Or_error.ok_exn
let merge styles = List.fold styles ~init:empty ~f:(fun result next ->
 Map.merge_skewed result next ~combine:(fun ~key:_ previous next ->
  Map.merge_skewed previous next ~combine:(fun ~key:_ _ next -> next)))
;;
let unset t ?(state=State.Base) name =
 let key=state_index state in
 let properties=Map.find t key |> Option.value ~default:Property.Name.Map.empty in
 Map.set t ~key ~data:(Map.set properties ~key:name ~data:None)
;;
module Expert = struct
 let color theme color = Theme.resolve theme color |> Or_error.map ~f:(fun rgba -> Wire.Color.Rgba rgba)
 let fill theme background = match Background.Expert.describe background with
 | Solid c -> let%map.Or_error c=color theme c in Wire.Fill.Solid c
 | Linear_gradient (angle,(from,start),(to_,stop)) ->
   let%bind.Or_error from=color theme from in
   let%map.Or_error to_=color theme to_ in
   Wire.Fill.Linear_gradient (angle,from,start,to_,stop)
 ;;
 let shadow theme shadow =
  let {Shadow.Expert.color=c;offset_x;offset_y;blur;spread;inset}=Shadow.Expert.describe shadow in
  let%map.Or_error color=color theme c in
  {Wire.Shadow.color;offset_x;offset_y;blur;spread;inset}
 ;;
 let field theme = function
 | Property.Display v -> Ok (Wire.Field.Display (Display.to_int64 v))
 | Property.Visibility v -> Ok (Wire.Field.Visibility (Visibility.to_int64 v))
 | Property.Direction v -> Ok (Wire.Field.Direction (Direction.to_int64 v))
 | Property.Wrap v -> Ok (Wire.Field.Wrap (Wrap.to_int64 v))
 | Property.Grow v -> Ok (Wire.Field.Grow (v))
 | Property.Shrink v -> Ok (Wire.Field.Shrink (v))
 | Property.Basis v -> Ok (Wire.Field.Basis (Length.Expert.to_wire v))
 | Property.Align_items v -> Ok (Wire.Field.Align_items (Align.to_int64 v))
 | Property.Align_self v -> Ok (Wire.Field.Align_self (Align.to_int64 v))
 | Property.Align_content v -> Ok (Wire.Field.Align_content (Distribution.to_int64 v))
 | Property.Justify_content v -> Ok (Wire.Field.Justify_content (Distribution.to_int64 v))
 | Property.Row_gap v -> Ok (Wire.Field.Row_gap (Length.Expert.to_wire v))
 | Property.Column_gap v -> Ok (Wire.Field.Column_gap (Length.Expert.to_wire v))
 | Property.Grid_columns v -> Ok (Wire.Field.Grid_columns (Int64.of_int v))
 | Property.Grid_rows v -> Ok (Wire.Field.Grid_rows (Int64.of_int v))
 | Property.Grid_column_minimum v -> Ok (Wire.Field.Grid_column_minimum (Grid_minimum.to_int64 v))
 | Property.Grid_row_minimum v -> Ok (Wire.Field.Grid_row_minimum (Grid_minimum.to_int64 v))
 | Property.Width v -> Ok (Wire.Field.Width (Length.Expert.to_wire v))
 | Property.Height v -> Ok (Wire.Field.Height (Length.Expert.to_wire v))
 | Property.Min_width v -> Ok (Wire.Field.Min_width (Length.Expert.to_wire v))
 | Property.Min_height v -> Ok (Wire.Field.Min_height (Length.Expert.to_wire v))
 | Property.Max_width v -> Ok (Wire.Field.Max_width (Length.Expert.to_wire v))
 | Property.Max_height v -> Ok (Wire.Field.Max_height (Length.Expert.to_wire v))
 | Property.Padding_top v -> Ok (Wire.Field.Padding_top (Length.Expert.to_wire v))
 | Property.Padding_right v -> Ok (Wire.Field.Padding_right (Length.Expert.to_wire v))
 | Property.Padding_bottom v -> Ok (Wire.Field.Padding_bottom (Length.Expert.to_wire v))
 | Property.Padding_left v -> Ok (Wire.Field.Padding_left (Length.Expert.to_wire v))
 | Property.Margin_top v -> Ok (Wire.Field.Margin_top (Length.Expert.to_wire v))
 | Property.Margin_right v -> Ok (Wire.Field.Margin_right (Length.Expert.to_wire v))
 | Property.Margin_bottom v -> Ok (Wire.Field.Margin_bottom (Length.Expert.to_wire v))
 | Property.Margin_left v -> Ok (Wire.Field.Margin_left (Length.Expert.to_wire v))
 | Property.Position v -> Ok (Wire.Field.Position (Position.to_int64 v))
 | Property.Top v -> Ok (Wire.Field.Top (Length.Expert.to_wire v))
 | Property.Right v -> Ok (Wire.Field.Right (Length.Expert.to_wire v))
 | Property.Bottom v -> Ok (Wire.Field.Bottom (Length.Expert.to_wire v))
 | Property.Left v -> Ok (Wire.Field.Left (Length.Expert.to_wire v))
 | Property.Background v -> let%map.Or_error v=fill theme v in Wire.Field.Background v
 | Property.Foreground v -> let%map.Or_error v=color theme v in Wire.Field.Foreground v
 | Property.Opacity v -> Ok (Wire.Field.Opacity (v))
 | Property.Border_top_width v -> Ok (Wire.Field.Border_top_width (v))
 | Property.Border_right_width v -> Ok (Wire.Field.Border_right_width (v))
 | Property.Border_bottom_width v -> Ok (Wire.Field.Border_bottom_width (v))
 | Property.Border_left_width v -> Ok (Wire.Field.Border_left_width (v))
 | Property.Top_left_radius v -> Ok (Wire.Field.Top_left_radius (v))
 | Property.Top_right_radius v -> Ok (Wire.Field.Top_right_radius (v))
 | Property.Bottom_left_radius v -> Ok (Wire.Field.Bottom_left_radius (v))
 | Property.Bottom_right_radius v -> Ok (Wire.Field.Bottom_right_radius (v))
 | Property.Border_color v -> let%map.Or_error v=color theme v in Wire.Field.Border_color v
 | Property.Shadows v -> let%map.Or_error v=List.map v ~f:(shadow theme) |> Or_error.all in Wire.Field.Shadows v
 | Property.Font_size v -> Ok (Wire.Field.Font_size (v))
 | Property.Font_family v -> Ok (Wire.Field.Font_family (v))
 | Property.Font_weight v -> Ok (Wire.Field.Font_weight (Int64.of_int v))
 | Property.Text_align v -> Ok (Wire.Field.Text_align (Text_align.to_int64 v))
 | Property.Line_height v -> Ok (Wire.Field.Line_height (Length.Expert.to_wire v))
 | Property.White_space v -> Ok (Wire.Field.White_space (White_space.to_int64 v))
 | Property.Text_overflow v -> Ok (Wire.Field.Text_overflow (Text_overflow.to_int64 v))
 | Property.Line_clamp v -> Ok (Wire.Field.Line_clamp (Int64.of_int v))
 | Property.Text_decoration v -> Ok (Wire.Field.Text_decoration (Text_decoration.to_int64 v))
 | Property.Overflow_x v -> Ok (Wire.Field.Overflow_x (Overflow.to_int64 v))
 | Property.Overflow_y v -> Ok (Wire.Field.Overflow_y (Overflow.to_int64 v))
 | Property.Cursor v -> Ok (Wire.Field.Cursor (Cursor.to_int64 v))
 | Property.Pointer_events v -> Ok (Wire.Field.Pointer_events (v))
 | Property.Padding _ | Margin _ | Gap _ | Border_width _ | Radius _ | Overflow _ -> assert false
 ;;
 let to_wire t ~theme =
  Map.to_alist t |> List.map ~f:(fun (state,properties) ->
   let%map.Or_error fields=Map.data properties |> List.filter_opt |> List.map ~f:(field theme) |> Or_error.all in
   if state=0 then Wire.Style.Fields fields else Wire.Style.State (Int64.of_int state,fields)) |> Or_error.all
 ;;
end

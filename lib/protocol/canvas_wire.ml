open Core

(* Geometry shared by the retained scene codec and pure admission checks.
   Units are logical pixels; device scale is applied by the native renderer. *)
let coordinate_limit = 1_000_000.
let hit_tolerance = 1e-7
let finite value = Float.is_finite value
let coordinate value = finite value && Float.(abs value <= coordinate_limit)

module Point = struct
  type t =
    { x : float
    ; y : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = coordinate t.x && coordinate t.y
end

module Path = struct
  module Command = struct
    type t =
      | Move of Point.t
      | Line of Point.t
      | Quadratic of Point.t * Point.t
      | Cubic of Point.t * Point.t * Point.t
      | Close
    [@@deriving bin_io, equal, sexp_of]

    let valid = function
      | Move point | Line point -> Point.valid point
      | Quadratic (control, point) -> Point.valid control && Point.valid point
      | Cubic (first, second, point) ->
        Point.valid first && Point.valid second && Point.valid point
      | Close -> true
    ;;
  end

  type t = Command.t list [@@deriving bin_io, equal, sexp_of]

  let max_commands = 4096

  (* Every contour starts with Move and contains a drawing command. Open
     contours are allowed for strokes; fill eligibility is checked separately. *)
  let valid t =
    let rec loop active segments has_drawing = function
      | [] -> has_drawing && ((not active) || segments > 0)
      | command :: rest ->
        Command.valid command
        &&
          (match command with
          | Move _ -> ((not active) || segments > 0) && loop true 0 has_drawing rest
          | Line _ | Quadratic _ | Cubic _ -> active && loop true (segments + 1) true rest
          | Close -> active && segments > 0 && loop false 0 has_drawing rest)
    in
    List.length t <= max_commands && loop false 0 false t
  ;;

  let is_closed t =
    let rec loop active = function
      | [] -> not active
      | Command.Move _ :: rest -> (not active) && loop true rest
      | Close :: rest -> loop false rest
      | (Line _ | Quadratic _ | Cubic _) :: rest -> loop active rest
    in
    valid t && loop false t
  ;;
end

module Rect = struct
  type t =
    { x : float
    ; y : float
    ; width : float
    ; height : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    coordinate t.x
    && coordinate t.y
    && coordinate t.width
    && coordinate t.height
    && Float.(t.width > 0. && t.height > 0.)
    && coordinate (t.x +. t.width)
    && coordinate (t.y +. t.height)
  ;;

  let contains t (point : Point.t) =
    Float.(
      point.x >= t.x -. hit_tolerance
      && point.y >= t.y -. hit_tolerance
      && point.x <= t.x +. t.width +. hit_tolerance
      && point.y <= t.y +. t.height +. hit_tolerance)
  ;;

  let intersect t other =
    let x = Float.max t.x other.x
    and y = Float.max t.y other.y in
    let right = Float.min (t.x +. t.width) (other.x +. other.width) in
    let bottom = Float.min (t.y +. t.height) (other.y +. other.height) in
    if Float.(right <= x || bottom <= y)
    then None
    else Some { x; y; width = right -. x; height = bottom -. y }
  ;;
end

module Transform = struct
  type t =
    { a : float
    ; b : float
    ; c : float
    ; d : float
    ; tx : float
    ; ty : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let identity = { a = 1.; b = 0.; c = 0.; d = 1.; tx = 0.; ty = 0. }
  let determinant t = (t.a *. t.d) -. (t.b *. t.c)

  let valid t =
    List.for_all [ t.a; t.b; t.c; t.d ] ~f:(fun value ->
      finite value && Float.(abs value <= 1000.))
    && coordinate t.tx
    && coordinate t.ty
    && Float.(abs (determinant t) >= 1e-8)
  ;;

  let apply t (point : Point.t) : Point.t =
    { x = (t.a *. point.x) +. (t.c *. point.y) +. t.tx
    ; y = (t.b *. point.x) +. (t.d *. point.y) +. t.ty
    }
  ;;

  let compose t local =
    { a = (t.a *. local.a) +. (t.c *. local.b)
    ; b = (t.b *. local.a) +. (t.d *. local.b)
    ; c = (t.a *. local.c) +. (t.c *. local.d)
    ; d = (t.b *. local.c) +. (t.d *. local.d)
    ; tx = (t.a *. local.tx) +. (t.c *. local.ty) +. t.tx
    ; ty = (t.b *. local.tx) +. (t.d *. local.ty) +. t.ty
    }
  ;;

  let inverse t =
    if not (valid t)
    then None
    else (
      let determinant = determinant t in
      Some
        { a = t.d /. determinant
        ; b = Float.neg t.b /. determinant
        ; c = Float.neg t.c /. determinant
        ; d = t.a /. determinant
        ; tx = ((t.c *. t.ty) -. (t.d *. t.tx)) /. determinant
        ; ty = ((t.b *. t.tx) -. (t.a *. t.ty)) /. determinant
        })
  ;;

  let unapply t (point : Point.t) =
    if not (valid t)
    then None
    else (
      let x = point.x -. t.tx
      and y = point.y -. t.ty in
      let determinant = determinant t in
      Some
        { Point.x = ((t.d *. x) -. (t.c *. y)) /. determinant
        ; y = ((t.a *. y) -. (t.b *. x)) /. determinant
        })
  ;;
  (* Inverses are calculation values and can exceed the admitted coefficient
     bound. They are not re-admitted as scene transforms implicitly. *)
end

module Hit_region = struct
  type t =
    | Rectangle of Rect.t
    | Ellipse of Rect.t
    | Polygon of Point.t list
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Rectangle rect | Ellipse rect -> Rect.valid rect
    | Polygon points ->
      let count = List.length points in
      count >= 3
      && count <= 256
      && List.for_all points ~f:Point.valid
      &&
        (match points with
        | [] -> false
        | (first : Point.t) :: rest ->
          Option.exists
            (List.find rest ~f:(fun p -> not (Point.equal first p)))
            ~f:(fun (second : Point.t) ->
              List.exists rest ~f:(fun (p : Point.t) ->
                Float.(
                  abs
                    (((second.x -. first.x) *. (p.y -. first.y))
                     -. ((second.y -. first.y) *. (p.x -. first.x)))
                  > 1e-8))))
  ;;

  let on_edge (point : Point.t) (a : Point.t) (b : Point.t) =
    let dx = b.x -. a.x
    and dy = b.y -. a.y in
    let cross = ((point.x -. a.x) *. dy) -. ((point.y -. a.y) *. dx) in
    let epsilon = hit_tolerance in
    Float.(
      abs cross <= epsilon *. max (abs dx) (abs dy)
      && point.x >= min a.x b.x -. epsilon
      && point.x <= max a.x b.x +. epsilon
      && point.y >= min a.y b.y -. epsilon
      && point.y <= max a.y b.y +. epsilon)
  ;;

  let contains t (point : Point.t) =
    match t with
    | Rectangle rect -> Rect.contains rect point
    | Ellipse rect ->
      let x =
        (point.x -. rect.x -. (rect.width /. 2.)) /. ((rect.width /. 2.) +. hit_tolerance)
      in
      let y =
        (point.y -. rect.y -. (rect.height /. 2.))
        /. ((rect.height /. 2.) +. hit_tolerance)
      in
      Float.((x *. x) +. (y *. y) <= 1.)
    | Polygon points ->
      (match List.last points with
       | None -> false
       | Some last ->
         let _, inside, boundary =
           List.fold
             points
             ~init:(last, false, false)
             ~f:(fun ((a : Point.t), inside, boundary) (b : Point.t) ->
               let crosses =
                 Bool.( <> ) Float.(a.y > point.y) Float.(b.y > point.y)
                 && Float.(
                      point.x < ((b.x -. a.x) *. (point.y -. a.y) /. (b.y -. a.y)) +. a.x)
               in
               b, (if crosses then not inside else inside), boundary || on_edge point a b)
         in
         boundary || inside)
  ;;

  let hit t ~transform ~clips point =
    let in_clip =
      match clips with
      | [] -> true
      | first :: rest ->
        List.fold rest ~init:(Some first) ~f:(fun intersection clip ->
          Option.bind intersection ~f:(fun current -> Rect.intersect current clip))
        |> Option.exists ~f:(fun clip -> Rect.contains clip point)
    in
    in_clip && Option.exists (Transform.unapply transform point) ~f:(contains t)
  ;;
end

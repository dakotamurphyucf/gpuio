open Core

module Direction = struct
  type t =
    | Increase
    | Decrease
  [@@deriving bin_io, equal, sexp_of]
end

module Domain = struct
  type t =
    { min : float
    ; max : float
    ; step : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Float.is_finite t.min
    && Float.is_finite t.max
    && Float.is_finite t.step
    && Float.(t.min <= t.max && t.step > 0.)
    && (Float.equal t.min t.max
        ||
        let span = t.max -. t.min in
        Float.is_finite span
        && Float.(span /. t.step <= 1099511627776.)
        && Float.(t.step >= 1.7763568394002505e-15 *. max (abs t.min) (abs t.max)))
  ;;

  let zero value = if Float.equal value 0. then 0. else value
  let contains t value = Float.is_finite value && Float.(value >= t.min && value <= t.max)

  let point t index =
    if Float.(index <= 0.)
    then zero t.min
    else if Float.(index >= (t.max -. t.min) /. t.step)
    then zero t.max
    else zero (Float.max t.min (Float.min t.max (t.min +. (index *. t.step))))
  ;;

  let normalize t value =
    if not (valid t && Float.is_finite value)
    then None
    else if Float.(value <= t.min)
    then Some (zero t.min)
    else if Float.(value >= t.max)
    then Some (zero t.max)
    else (
      let index = Float.round_down ((value -. t.min) /. t.step) in
      let lower = point t index
      and upper = point t (index +. 1.) in
      Some (if Float.(abs (value -. lower) < abs (upper -. value)) then lower else upper))
  ;;

  let advance t value ~direction =
    Option.map (normalize t value) ~f:(fun value ->
      let index = (value -. t.min) /. t.step in
      let first, delta, endpoint, progresses =
        match direction with
        | Direction.Increase ->
          Float.round_down index, 1., t.max, fun next -> Float.(next > value)
        | Decrease -> Float.round_up index, -1., t.min, fun next -> Float.(next < value)
      in
      (* Four candidates bound quotient/reconstruction rounding without a walk
         proportional to the number of steps. Domain precision guards keep
         adjacent regular points distinct. *)
      let rec next offset =
        if offset = 4
        then zero endpoint
        else (
          let candidate = point t (first +. (Float.of_int offset *. delta)) in
          if progresses candidate then candidate else next (offset + 1))
      in
      next 0)
  ;;
end

module Draft = struct
  module Error = struct
    type t =
      | Syntax
      | Non_finite
      | Too_long
    [@@deriving equal, sexp_of]
  end

  type t =
    | Empty
    | Incomplete
    | Invalid of Error.t
    | Valid of float
    | Out_of_range of float
  [@@deriving equal, sexp_of]

  let whitespace = function
    | ' ' | '\t' | '\r' | '\n' | '\011' | '\012' -> true
    | _ -> false
  ;;

  let parse domain source =
    if String.length source > 4096
    then Invalid Too_long
    else (
      let text = String.strip source ~drop:whitespace in
      let length = String.length text in
      if length = 0
      then Empty
      else (
        let index = ref 0 in
        let sign () =
          if
            !index < length
            && (Char.equal text.[!index] '+' || Char.equal text.[!index] '-')
          then Int.incr index
        in
        let digits () =
          let start = !index in
          while !index < length && Char.is_digit text.[!index] do
            Int.incr index
          done;
          !index - start
        in
        sign ();
        let before = digits () in
        let after =
          if !index < length && Char.equal text.[!index] '.'
          then (
            Int.incr index;
            digits ())
          else 0
        in
        if before + after = 0
        then if !index = length then Incomplete else Invalid Syntax
        else (
          let missing_exponent =
            if
              !index < length
              && (Char.equal text.[!index] 'e' || Char.equal text.[!index] 'E')
            then (
              Int.incr index;
              sign ();
              digits () = 0)
            else false
          in
          if !index <> length
          then Invalid Syntax
          else if missing_exponent
          then Incomplete
          else (
            match Option.try_with (fun () -> Float.of_string text) with
            | None -> Invalid Syntax
            | Some value when not (Float.is_finite value) -> Invalid Non_finite
            | Some value ->
              let value = Domain.zero value in
              if Domain.contains domain value then Valid value else Out_of_range value))))
  ;;
end

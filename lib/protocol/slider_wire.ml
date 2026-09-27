open Core

module Axis = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving bin_io, equal, sexp_of]
end

module Scale = struct
  type t =
    | Linear
    | Logarithmic
  [@@deriving bin_io, equal, sexp_of]
end

module Thumb = struct
  type t =
    | Single
    | Lower
    | Upper
  [@@deriving bin_io, equal, sexp_of]
end

module Value = struct
  type t =
    | Single of float
    | Range of
        { lower : float
        ; upper : float
        }
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Single value -> Float.is_finite value
    | Range { lower; upper } ->
      Float.is_finite lower && Float.is_finite upper && Float.(lower <= upper)
  ;;

  let supports t thumb =
    match t, thumb with
    | Single _, Thumb.Single | Range _, (Lower | Upper) -> true
    | Single _, (Lower | Upper) | Range _, Single -> false
  ;;

  let same_mode a b =
    match a, b with
    | Single _, Single _ | Range _, Range _ -> true
    | _ -> false
  ;;
end

module Config = struct
  type t =
    { domain : Numeric_wire.Domain.t
    ; label : string
    ; lower_label : string
    ; upper_label : string
    ; axis : Axis.t
    ; scale : Scale.t
    ; disabled : bool
    ; read_only : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid_text s =
    String.length s <= 4096
    && (not (String.is_empty (String.strip s ~drop:Numeric_wire.Draft.whitespace)))
    && Stdlib.String.is_valid_utf_8 s
    && not (String.contains s '\000')
  ;;

  let valid t =
    Numeric_wire.Domain.valid t.domain
    && valid_text t.label
    && valid_text t.lower_label
    && valid_text t.upper_label
    &&
    match t.scale with
    | Linear -> true
    | Logarithmic -> Float.(t.domain.min > 0.)
  ;;
end

module Snapshot = struct
  type t =
    { revision : int64
    ; value : Value.t
    ; committed : Value.t
    ; dragging : Thumb.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.revision >= 0L)
    && Value.valid t.value
    && Value.valid t.committed
    && Value.same_mode t.value t.committed
    &&
    match t.dragging, t.value, t.committed with
    | None, value, committed -> Value.equal value committed
    | Some Single, Single _, Single _ -> Int64.(t.revision > 0L)
    | Some Lower, Range { upper; _ }, Range { upper = previous; _ }
    | Some Upper, Range { lower = upper; _ }, Range { lower = previous; _ } ->
      Int64.(t.revision > 0L) && Float.equal upper previous
    | _ -> false
  ;;
end

module Source = struct
  type t =
    | Pointer
    | Keyboard
    | Accessibility
  [@@deriving bin_io, equal, sexp_of]
end

module Cancel_reason = struct
  type t =
    | Escape
    | Configuration_changed
    | Disabled
    | Read_only
    | Hidden
    | Modal
    | Window_inactive
    | Unmounted
    | Programmatic
    | Interrupted
  [@@deriving bin_io, equal, sexp_of]
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Drag_started of Snapshot.t
    | Preview of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving bin_io, equal, sexp_of]

  let snapshot = function
    | Observed s | Drag_started s | Preview s | Committed (_, s) | Cancelled (_, s) -> s
  ;;

  let valid t =
    let s = snapshot t in
    Snapshot.valid s
    &&
    match t with
    | Observed _ -> true
    | Drag_started _ -> Option.is_some s.dragging && Value.equal s.value s.committed
    | Preview _ -> Option.is_some s.dragging
    | Committed _ | Cancelled _ -> Option.is_none s.dragging && Int64.(s.revision > 0L)
  ;;
end

module Command = struct
  type t =
    | Replace of
        { value : Value.t
        ; if_revision : int64 option
        }
    | Cancel_drag
    | Focus of Thumb.t
    | Read_snapshot
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Replace { value; if_revision } ->
      Value.valid value && Option.for_all if_revision ~f:(fun r -> Int64.(r >= 0L))
    | Cancel_drag | Focus _ | Read_snapshot -> true
  ;;
end

module Error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_slider
    | Stale_revision
    | Wrong_mode
    | Wrong_thumb
    | Disabled
    | Focus_blocked
    | Busy
    | Invalid_value
    | Invalid_config
    | Limit_exceeded
    | Read_only
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Applied of Snapshot.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Applied snapshot -> Snapshot.valid snapshot
    | Failed _ -> true
  ;;
end

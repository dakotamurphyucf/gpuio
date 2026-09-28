open Core

module Button = struct
  type t =
    | Left
    | Right
    | Middle
    | Back
    | Forward
  [@@deriving bin_io, equal, sexp_of]
end

module Cancel_reason = struct
  type t =
    | Escape
    | Hidden
    | Blocked
    | Disabled
    | Reconfigured
    | Capture_lost
    | Window_inactive
    | Removed
  [@@deriving bin_io, equal, sexp_of]
end

module Phase = struct
  type t =
    | Started
    | Moved
    | Released
    | Cancelled of Cancel_reason.t
  [@@deriving bin_io, equal, sexp_of]
end

module Modifiers = struct
  type t =
    { shift : bool
    ; control : bool
    ; alt : bool
    ; command : bool
    ; function_ : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; button : Button.t
    ; disabled : bool
    ; prevent_default : bool
    ; stop_propagation : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Sample = struct
  type t =
    { gesture : int64
    ; phase : Phase.t
    ; button : Button.t
    ; window_x : float
    ; window_y : float
    ; local_x : float
    ; local_y : float
    ; modifiers : Modifiers.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

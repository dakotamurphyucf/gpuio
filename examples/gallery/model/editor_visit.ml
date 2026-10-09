open Core

type t =
  { mutable phase : [ `Fresh | `Active | `Retired ]
  ; mutable revision : int64 option
  }

let create () = { phase = `Fresh; revision = None }

let activate t =
  match t.phase with
  | `Fresh | `Active -> t.phase <- `Active
  | `Retired -> invalid_arg "An editor visit cannot be reactivated"
;;

let deactivate t = t.phase <- `Retired

let is_active t =
  match t.phase with
  | `Active -> true
  | `Fresh | `Retired -> false
;;

let observe t ~revision =
  if
    (not (is_active t))
    || Int64.(revision < 0L)
    || Option.exists t.revision ~f:(fun previous -> Int64.(revision <= previous))
  then false
  else (
    t.revision <- Some revision;
    true)
;;

open Core

module Token = struct
  type t = unit ref

  let equal = phys_equal
  let create () = ref ()
end

type t =
  { token : Token.t
  ; preference : Appearance.Preference.t
  ; profile : Theme_profile.t option
  }

let initial () = { token = Token.create (); preference = Explicit Dark; profile = None }
let token t = t.token
let preference t = t.preference
let profile t = t.profile
let choose _ preference = { token = Token.create (); preference; profile = None }
let resolve t ~native = Appearance.Preference.resolve t.preference ~native

let complete t ~token result =
  if not (Token.equal t.token token)
  then t
  else (
    match result with
    | Error _ -> t
    | Ok profile ->
      { token = Token.create ()
      ; preference = Explicit (Theme_profile.appearance profile)
      ; profile = Some profile
      })
;;

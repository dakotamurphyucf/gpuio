open Core
open Gpuio

(** A public-API recipe; the supplied list remains the only scroll owner. *)
val is_away : Virtual_list.Viewport.t option -> bool

(** Absolute overlays never change list layout. The jump subtree is immediately
    inert when unavailable, while its native wrapper may finish fading. Omit
    [fade] to disable the bottom fade. [motion=false] settles without a tween.
    The caller supplies a button with its generation-bound jump action and style,
    fitting the 48-pixel-high clipped slot. The hidden slot settles below the
    viewport so it cannot shield pointer input over the messages.
    Native reduced-motion policy still applies when [motion=true]. *)
val view
  :  viewport:Virtual_list.Viewport.t option
  -> jump_enabled:bool
  -> motion:bool
  -> fade:Color.t option
  -> jump:'action View.t
  -> 'action View.t
  -> 'action View.t

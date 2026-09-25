open Core
module Registration = Canvas_registry.Registration
module Error = Canvas_registry.Error

type t = Registration.t

let create = App.Expert.register_canvas
let handle = Registration.handle
let scene = Registration.scene
let is_published = Registration.is_published
let set = Registration.set
let reset = Registration.reset
let error = Registration.error
let release = Registration.release
let is_released = Registration.is_released

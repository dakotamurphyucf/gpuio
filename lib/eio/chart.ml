open Core
module Registration = Chart_registry.Registration
module Error = Chart_registry.Error

type t = Registration.t

let create = App.Expert.register_chart
let handle = Registration.handle
let data = Registration.data
let is_published = Registration.is_published
let set = Registration.set
let reset = Registration.reset
let error = Registration.error
let release = Registration.release
let is_released = Registration.is_released

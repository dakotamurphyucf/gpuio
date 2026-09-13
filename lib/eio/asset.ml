module Source = Gpuio.Asset.Source
module Format = Gpuio.Asset.Format
module Error = Asset_registry.Error

type t = Asset_registry.Registration.t

let register = App.Expert.register_asset
let release = Asset_registry.Registration.release
let is_released = Asset_registry.Registration.is_released

module Expert = Asset_registry.Registration.Expert

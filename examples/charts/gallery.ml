open Core
module Samples = Gpuio_chart_samples
module Preset = Samples.Preset

(* The standalone chart exercise retains its numeric CLI/test selection. The
   shared sample catalog itself uses a closed family type. *)
let family index = Samples.Family.of_index index |> Or_error.ok_exn
let names = Array.of_list (List.map Samples.Family.all ~f:Samples.Family.label)
let edge_data index = Samples.edge_data (family index)
let preset_data preset index phase = Samples.preset_data_exn preset (family index) phase
let description preset index = Samples.description preset (family index)
let describe_selection = Samples.describe_selection

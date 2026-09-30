open Core
module B = Bonsai.Cont
module Binding = Gpuio.Command_binding

let component ?key ?(style = B.return Gpuio.Style.empty) ~config ~f graph =
  (* Only the transient observation model belongs to the configuration visit.
     Keep the caller's child computations outside this association so changing
     a lookup cannot reset an editor or other application state. Config's derived
     sexp is an injective bounded key; it is computed only when config changes. *)
  let configurations =
    B.map config ~f:(fun config ->
      Map.singleton
        (module String)
        (Binding.Config.sexp_of_t config |> Sexp.to_string_mach)
        config)
  in
  let visits =
    Managed_rows.assoc
      (module String)
      configurations
      ~f:(fun _ _ lifetime graph ->
        let observation, set_observation =
          B.state_opt ~equal:Binding.Observation.equal graph
        in
        let open B.Let_syntax in
        let%arr observation = observation
        and set_observation = set_observation
        and lifetime = lifetime in
        ( observation
        , fun observation ->
            Managed_rows.Lifetime.guard lifetime (set_observation (Some observation)) ))
      graph
  in
  let visit = B.map visits ~f:(fun visits -> Map.min_elt_exn visits |> snd) in
  let observation = B.map visit ~f:fst in
  let children = f observation graph in
  let open B.Let_syntax in
  let%arr config = config
  and style = style
  and visit = visit
  and children = children in
  Gpuio.View.command_binding_scope ?key ~style ~config ~on_update:(snd visit) children
;;

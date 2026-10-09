open Core

module Config = struct
  type t =
    { loading : Loading.Config.t
    ; image : Image.Config.t option
    ; easing : Animation.Easing.t
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ?icon
        ?(easing = Animation.Easing.ease_in_out)
        ?(period = Time_ns.Span.of_ms 800.)
        ?(animated = true)
        ()
    =
    let open Or_error.Let_syntax in
    let%bind loading = Loading.Config.create ~kind:Spinner ~label ~period ~animated () in
    let%map image =
      Option.value_map icon ~default:(Ok None) ~f:(fun asset ->
        Icon.Config.create ~asset ~description:Image.Description.decorative ()
        |> Or_error.map ~f:(fun config -> Some (Icon.Expert.image config)))
    in
    { loading; image; easing }
  ;;
end

module Expert = struct
  let image (t : Config.t) = t.image
  let loading (t : Config.t) = t.loading

  let to_wire (t : Config.t) ~owner : Gpuio_protocol.Spinner_wire.Config.t =
    let { Gpuio_protocol.Loading_wire.Config.kind = _; label; animated; period_ms } =
      Loading.Expert.to_wire t.loading
    in
    { label
    ; animated
    ; period_ms
    ; easing = Animation.Expert.easing_to_wire t.easing
    ; source =
        Option.map t.image ~f:(fun image -> (Image.Expert.to_wire image ~owner).source)
    }
  ;;
end

open Core
module Chart = Gpuio.Chart
module Resource = Gpuio.Chart_resource
module W = Gpuio_protocol.Chart_view_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let source = Gpuio_protocol.Resource_id.create ~slot:7L ~generation:2L |> ok
let owner = Resource.Expert.Owner.create ()
let handle = Resource.Expert.handle ~owner source

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let style =
  Gpuio.Chart_style.Expert.of_wire
    { version = -2L
    ; palette = [ 1L; 2L ]
    ; axis_color = 3L
    ; grid_color = 4L
    ; label_color = 5L
    ; selection_color = 6L
    ; gradient_end = Some 7L
    ; stroke_width = 2.
    ; point_radius = 3.
    ; bar_radius = 4.
    ; area_opacity = 0.5
    ; node_labels = []
    ; inspection =
        Gpuio.Chart_inspection.Expert.to_wire
          Gpuio.Chart_inspection.default
          ~theme:Gpuio.Theme.default
        |> Or_error.ok_exn
    ; ordinal = None
    }
  |> ok
;;

let%expect_test "chart view owner and append-only envelopes match independent fixtures" =
  let config = Chart.Config.create ~data:handle ~label:"Chart 🦀" ~style () |> ok in
  let wire = Chart.Expert.to_wire config ~owner:(Some owner) in
  assert (W.Config.valid wire);
  assert wire.legend;
  assert (not wire.disabled);
  let disabled = Chart.Config.create ~data:handle ~disabled:true () |> ok in
  assert (Chart.Expert.to_wire disabled ~owner:(Some owner)).disabled;
  let hidden_legend = Chart.Config.create ~data:handle ~legend:false () |> ok in
  assert (not (Chart.Expert.to_wire hidden_legend ~owner:(Some owner)).legend);
  assert (Resource.equal (Chart.Config.data config) handle);
  assert (Option.is_none (Chart.Expert.to_wire config ~owner:None).source);
  assert (
    Option.is_none
      (Chart.Expert.to_wire config ~owner:(Some (Resource.Expert.Owner.create ()))).source);
  let bytes = Bin_prot.Utils.bin_dump W.Config.bin_writer_t wire |> Bigstring.to_string in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "chart-v4-node-labels-view.hex")
      |> String.strip
    in
    assert (String.equal (hex bytes) expected));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let message =
    Wire.Message.encode
      (Apply
         { window
         ; base = 0L
         ; revision = 1L
         ; operations =
             [ Create (node, Chart_view, "", Some handler); Set_chart (node, wire) ]
         })
    |> ok
  in
  assert (String.equal (hex message) ("0300010001020000013000010001370001" ^ hex bytes));
  let metrics : W.Metrics.t =
    { source_values = 10L
    ; retained_values = 5L
    ; mesh_vertices = 12L
    ; quads = 2L
    ; bytes = 100L
    }
  in
  let encoded =
    "\001\062\000\001\000\001\000\001\001\001\007\002\002\003\000\010\005\012\002\100"
  in
  assert (
    List.equal
      Wire.Event.equal
      (Wire.Event.decode encoded |> ok)
      [ Chart_event (window, node, handler, 1L, Some source, 2L, 3L, Ready metrics) ]);
  let event =
    Chart.Expert.event ~data_revision:2L ~data_generation:3L (Ready metrics) |> ok
  in
  assert (
    Chart.Observation.equal
      event.observation
      (Ready
         { source_values = 10
         ; retained_values = 5
         ; mesh_vertices = 12
         ; quads = 2
         ; bytes = 100
         }));
  for len = 0 to String.length encoded - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix encoded len)))
  done;
  assert (Result.is_error (Wire.Event.decode (encoded ^ "\000")));
  print_endline
    "owner fencing; kind 48, operation 55, event 62; fixture and truncation checks pass";
  [%expect
    {| owner fencing; kind 48, operation 55, event 62; fixture and truncation checks pass |}]
;;

let%expect_test "chart configuration and observations reject invalid boundaries" =
  List.iter
    [ ""; " \t\011\012"; "\000"; "\255"; "chart\n"; String.make 1025 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (Chart.Config.create ~data:handle ~label ())));
  assert (Result.is_ok (Chart.Config.create ~data:handle ~label:(String.make 1024 'x') ()));
  let metrics : W.Metrics.t =
    { source_values = 1L
    ; retained_values = 1L
    ; mesh_vertices = 1L
    ; quads = 1L
    ; bytes = 1L
    }
  in
  let valid revision generation observation =
    Result.is_ok
      (Chart.Expert.event ~data_revision:revision ~data_generation:generation observation)
  in
  List.iter
    [ 0L, 0L; 0L, 1L; 1L, 0L; -1L, 1L ]
    ~f:(fun (revision, generation) ->
      assert (not (valid revision generation (Ready metrics))));
  assert (valid 0L 0L (Failed Unavailable_data));
  List.iter
    [ { metrics with source_values = 100_001L }
    ; { metrics with retained_values = 2L }
    ; { metrics with mesh_vertices = 1_000_001L }
    ; { metrics with quads = 300_001L }
    ; { metrics with bytes = 67_108_865L }
    ; { metrics with bytes = -1L }
    ]
    ~f:(fun m -> assert (not (valid 1L 1L (Ready m))));
  print_endline "labels, epochs and bounded preparation counts validated";
  [%expect {| labels, epochs and bounded preparation counts validated |}]
;;

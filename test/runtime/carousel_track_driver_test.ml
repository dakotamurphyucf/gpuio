open Core
open Gpuio
open Gpuio_protocol
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module C = Carousel_track
module Driver = Gpuio_runtime_core.Window_driver
module W = Carousel_track_wire

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok
let item label = C.Item.create ~id:(C.Id.of_string label |> ok) ~label () |> ok

module Action = struct
  type t =
    | Request of C.Request.t
    | Reverse
    | Disable
end

let component graph =
  let model, dispatch =
    B.state_machine0
      ~default_model:
        (C.create
           ~auto_advance:
             (C.Auto_advance.create ~interval:(Time_ns.Span.of_sec 4.) () |> ok)
           (List.map [ "Alpha"; "Beta"; "Gamma" ] ~f:item)
         |> ok)
      ~apply_action:(fun _ model -> function
         | Action.Request request -> C.apply_request model request |> ok
         | Reverse -> C.with_items model (List.rev (C.items model)) |> ok
         | Disable -> C.with_disabled model true |> ok)
      graph
  in
  let open B.Let_syntax in
  let%arr model = model
  and dispatch = dispatch in
  V.column
    [ V.carousel_track
        model
        ~label:"Driver cards"
        ~on_request:(fun request -> dispatch (Request request))
        ~content:(fun item -> [ V.text (C.Item.label item) ])
        ()
    ; V.button ~on_click:(dispatch Reverse) "Reverse"
    ; V.button ~on_click:(dispatch Disable) "Disable"
    ]
;;

let cycle driver = Driver.cycle driver ~now:Time_ns.epoch |> ok

let accept driver =
  match Driver.next_message driver with
  | Some (Wire.Message.Apply tx) ->
    Driver.submitted driver;
    Driver.acknowledge driver ~revision:tx.revision |> ok;
    tx.operations
  | Some _ | None -> assert false
;;

let assert_retained ops =
  assert (
    not
      (List.exists ops ~f:(function
         | Wire.Op.Create _ | Remove _ -> true
         | _ -> false)))
;;

let config ops =
  List.find_map_exn ops ~f:(function
    | Wire.Op.Set_carousel_track (_, config) -> Some config
    | _ -> None)
;;

let%expect_test "measured carousel reduces ordered native requests through Bonsai" =
  let driver = Driver.create window ~start:Time_ns.epoch ~theme:Theme.default component in
  cycle driver;
  let initial = accept driver in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (node, Carousel_track, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let press label =
    let node, handler =
      List.find_map_exn initial ~f:(function
        | Wire.Op.Create (node, Button, name, Some handler) when String.equal name label
          -> Some (node, handler)
        | _ -> None)
    in
    Driver.dispatch driver (Press (window, node, handler, Driver.revision driver))
  in
  let request value =
    Driver.dispatch
      driver
      (Carousel_track_requested (window, node, handler, Driver.revision driver, value))
  in
  let layout lineage epoch =
    request
      (W.Request.Layout
         { lineage; epoch; stops = Some { canonical = [ 0L; 1L; 2L ]; looping = Finite } })
  in
  layout 0L 0L;
  cycle driver;
  let measured = accept driver in
  assert_retained measured;
  assert (
    not
      (List.exists measured ~f:(function
         | Wire.Op.Set_carousel_track _ -> true
         | _ -> false)));
  request Next;
  request Next;
  request
    (Auto_next { revision = 0L; geometry_epoch = 0L; from = "Alpha"; target = "Beta" });
  cycle driver;
  let advanced = accept driver in
  assert_retained advanced;
  let advanced = config advanced in
  assert (Option.equal Int64.equal advanced.carousel.selected (Some 2L));
  assert (Int64.equal advanced.carousel.revision 2L);
  print_endline "two ordered Next requests; stale automatic proposal ignored";
  layout 0L 0L;
  cycle driver;
  assert (Option.is_none (Driver.next_message driver));
  press "Reverse";
  cycle driver;
  let reversed = accept driver in
  assert_retained reversed;
  let reversed = config reversed in
  assert (Int64.equal reversed.lineage 1L);
  assert (List.equal String.equal reversed.carousel.ids [ "Gamma"; "Beta"; "Alpha" ]);
  assert (Option.equal Int64.equal reversed.carousel.selected (Some 0L));
  layout 0L 99L;
  cycle driver;
  assert (Option.is_none (Driver.next_message driver));
  layout 1L 1L;
  cycle driver;
  assert_retained (accept driver);
  print_endline "reorder retains owners and selection by ID; old lineage rejected";
  press "Disable";
  cycle driver;
  let disabled = accept driver in
  assert_retained disabled;
  assert (config disabled).carousel.disabled;
  request Next;
  cycle driver;
  assert (Option.is_none (Driver.next_message driver));
  for _ = 1 to 100 do
    cycle driver;
    assert (Option.is_none (Driver.next_message driver))
  done;
  Driver.close driver;
  request Next;
  print_endline "disabled input ignored; idle cycles emit nothing; close fences callbacks";
  [%expect
    {|
    two ordered Next requests; stale automatic proposal ignored
    reorder retains owners and selection by ID; old lineage rejected
    disabled input ignored; idle cycles emit nothing; close fences callbacks
    |}]
;;

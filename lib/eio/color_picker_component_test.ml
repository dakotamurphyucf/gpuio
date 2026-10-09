open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module C = Gpuio.Color_input
module Picker = Color_picker_component
module W = Gpuio_protocol.Wire
module Driver = Gpuio_runtime_core.Window_driver
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let overlay = Gpuio.Overlay.Config.create ~label:"Color popup" () |> ok

let config ?(disabled = false) () =
  C.Config.create ~labels:(C.Labels.english ~control:"Accent" |> ok) ~disabled () |> ok
;;

type harness =
  { driver : Driver.t
  ; initial : W.Op.t list
  ; config : C.Config.t B.Expert.Var.t
  ; label : string B.Expert.Var.t
  ; appearance : C.Appearance.t B.Expert.Var.t
  ; latest : Picker.t option ref
  ; commits : Gpuio.Color_value.Value.t Queue.t
  }

let cycle t =
  Driver.cycle t.driver ~now:Time_ns.epoch |> ok;
  match Driver.next_message t.driver with
  | Some (W.Message.Apply tx) ->
    Driver.submitted t.driver;
    Driver.acknowledge t.driver ~revision:tx.revision |> ok;
    tx.operations
  | None -> []
  | Some _ -> assert false
;;

let current t = Option.value_exn !(t.latest)

let run t action =
  Driver.schedule t.driver action;
  cycle t
;;

let create () =
  let config = B.Expert.Var.create (config ()) in
  let label = B.Expert.Var.create "Pick an accent" in
  let appearance = B.Expert.Var.create C.Appearance.default in
  let latest = ref None
  and commits = Queue.create () in
  let component graph =
    let open B.Let_syntax in
    let picker =
      Picker.create
        (fun _ _ -> assert false)
        ~config:(B.Expert.Var.value config)
        ~value:(B.return Gpuio.Color_value.Value.Empty)
        ~on_change:
          (B.return (fun color -> E.of_thunk (fun () -> Queue.enqueue commits color)))
        graph
    in
    let%arr picker = picker
    and label = B.Expert.Var.value label
    and appearance = B.Expert.Var.value appearance in
    latest := Some picker;
    Picker.view_with_trigger
      ~appearance
      ~overlay
      ~accessible_name:"Choose accent"
      ~trigger:(V.row [ V.text label ])
      picker
    |> ok
  in
  let driver =
    Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
  in
  let t = { driver; initial = []; config; label; appearance; latest; commits } in
  { t with initial = cycle t }
;;

let button operations =
  List.find_map_exn operations ~f:(function
    | W.Op.Create (id, Button, "Choose accent", Some handler) -> Some (id, handler)
    | _ -> None)
;;

let click t (node, handler) =
  Driver.dispatch
    t.driver
    (W.Event.Press (window, node, handler, Driver.revision t.driver));
  cycle t
;;

let input operations =
  List.find_map_exn operations ~f:(function
    | W.Op.Set_color_input (node, _, _) -> Some node
    | _ -> None)
;;

let%expect_test
    "rich color trigger routes open/cancel and refreshes without remounting its draft"
  =
  let t = create () in
  assert (
    List.exists t.initial ~f:(function
      | W.Op.Set_popover (_, true) -> true
      | _ -> false));
  let trigger = button t.initial in
  let first = input (click t trigger) in
  assert (Picker.is_open (current t));
  B.Expert.Var.set t.label "An updated color caption";
  let changed = cycle t in
  assert (
    not
      (List.exists changed ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  assert (Picker.is_open (current t));
  let old = current t in
  ignore (run t (Picker.cancel old) : W.Op.t list);
  let second = input (run t (Picker.open_popup (current t))) in
  assert (not (Gpuio_protocol.Node_id.equal first second));
  ignore (run t (Picker.cancel old) : W.Op.t list);
  assert (Picker.is_open (current t) && Queue.is_empty t.commits);
  B.Expert.Var.set t.config (config ~disabled:true ());
  ignore (cycle t : W.Op.t list);
  assert (not (Picker.is_open (current t)));
  ignore (click t trigger : W.Op.t list);
  assert ((not (Picker.is_open (current t))) && Queue.is_empty t.commits);
  print_endline
    "one native trigger; content refresh retains draft; new opening and disabled policy \
     fence stale actions";
  [%expect
    {| one native trigger; content refresh retains draft; new opening and disabled policy fence stale actions |}]
;;

let%expect_test "rich color trigger rejects nested interaction and validates names" =
  let t = create () in
  let view content name =
    Picker.view_with_trigger ~overlay ~accessible_name:name ~trigger:content (current t)
  in
  assert (Result.is_error (view (V.button ~on_click:E.Ignore "Clear") "Color"));
  List.iter
    [ ""; "  "; "\000"; "\255"; String.make 1025 'x' ]
    ~f:(fun name -> assert (Result.is_error (view (V.text "Color") name)));
  assert (
    Result.is_ok (view (V.row [ V.text "Accent"; V.text "#AABBCC" ]) "Choose accent"));
  assert ((not (Picker.is_open (current t))) && Queue.is_empty t.commits);
  print_endline "validated passive presentation; view construction has no picker effects";
  [%expect {| validated passive presentation; view construction has no picker effects |}]
;;

let%expect_test "popup palette layout and appearance update the current draft owner" =
  let t = create () in
  let first = input (run t (Picker.open_popup (current t))) in
  let presentation ops =
    List.filter_map ops ~f:(function
      | W.Op.Set_color_presentation (node, p) ->
        assert (Gpuio_protocol.Node_id.equal node first);
        Some p
      | W.Op.Create _ | W.Op.Remove _ -> assert false
      | _ -> None)
  in
  B.Expert.Var.set t.appearance (C.Appearance.create ~swatch_size:36. () |> ok);
  let styled = cycle t in
  assert (List.length (presentation styled) = 1);
  assert (
    not
      (List.exists styled ~f:(function
         | W.Op.Set_color_input _ -> true
         | _ -> false)));
  let entries =
    [ C.Palette_entry.create
        ~label:"Blue"
        ~color:(Gpuio.Color_value.Rgba.of_hex "#1122FF" |> ok)
      |> ok
    ]
  in
  let labels = C.Labels.english ~control:"Accent" |> ok in
  B.Expert.Var.set
    t.config
    (C.Config.create
       ~labels
       ~palette_sections:[ C.Palette_section.featured ~label:"Favorites" entries |> ok ]
       ()
     |> ok);
  let grouped = cycle t in
  assert (Gpuio_protocol.Node_id.equal (input grouped) first);
  assert (List.length (presentation grouped) = 1);
  let panels = C.Panels.tabs ~palette_label:"Palette" ~channels_label:"HSLA" () |> ok in
  B.Expert.Var.set t.appearance (C.Appearance.create ~panels () |> ok);
  (match presentation (cycle t) with
   | [ Some { panels = Tabs { initial = Palette; _ }; _ } ] -> ()
   | _ -> assert false);
  assert (Picker.is_open (current t) && Queue.is_empty t.commits);
  B.Expert.Var.set t.config (C.Config.create ~labels ~palette:entries () |> ok);
  B.Expert.Var.set t.appearance C.Appearance.default;
  (match presentation (cycle t) with
   | [ None ] -> ()
   | _ -> assert false);
  assert (Picker.is_open (current t) && Queue.is_empty t.commits);
  print_endline
    "style/group/reset retain popup input identity and leave application value \
     uncommitted";
  [%expect
    {| style/group/reset retain popup input identity and leave application value uncommitted |}]
;;

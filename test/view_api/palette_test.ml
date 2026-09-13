open Core
open Gpuio
open Gpuio_protocol
module Wire = Gpuio_protocol.Wire

let id name = Command.Id.of_string name |> Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let node slot = Node_id.create ~slot ~generation:1L |> Or_error.ok_exn
let handler slot = Handler_id.create ~slot ~generation:1L |> Or_error.ok_exn

let config =
  Command_palette.Config.create
    ~label:"Actions"
    ~placeholder:"Find…"
    ~commands:[ id "run"; id "copy" ]
    ~dismiss_on_outside_pointer:false
    ()
  |> Or_error.ok_exn
;;

let commands =
  Command.Registry.create
    [ Command.create
        ~id:(id "run")
        ~label:"Run"
        ~checked:true
        ~on_invoke:(fun () -> "run")
        ()
      |> Or_error.ok_exn
    ; Command.native ~id:(id "copy") ~label:"Copy" Copy |> Or_error.ok_exn
    ]
  |> Or_error.ok_exn
;;

let%expect_test "palette bounds and dismissal intents" =
  assert (Result.is_error (Command_palette.Config.create ~label:" " ~commands:[] ()));
  assert (
    Result.is_error
      (Command_palette.Config.create ~label:"Actions" ~placeholder:"\000" ~commands:[] ()));
  assert (
    Result.is_error
      (Command_palette.Config.create ~label:"Actions" ~commands:[ id "run"; id "run" ] ()));
  assert (
    Result.is_error
      (Command_palette.Config.create
         ~label:"Actions"
         ~commands:(List.init 1025 ~f:(fun i -> id (Int.to_string i)))
         ()));
  assert (Option.is_none (Command_palette.Expert.dismissal config Outside_pointer));
  assert (Option.is_none (Command_palette.Expert.dismissal config (Selected "missing")));
  assert (Option.is_some (Command_palette.Expert.dismissal config (Selected "copy")));
  [%expect {| |}]
;;

let%expect_test "palette command references, callback refresh and stale dismissal" =
  let reconciler = Reconciler.create window in
  let view callback = View.command_palette ~config ~on_dismiss:callback () in
  let callback prefix reason =
    prefix ^ Sexp.to_string (Command_palette.Dismissal.sexp_of_t reason)
  in
  let shared = view (callback "first:") in
  let prepare view = Reconciler.prepare reconciler ~theme:Theme.default (Some view) in
  let first = prepare (View.command_scope ~commands [ shared ]) |> Or_error.ok_exn in
  let palette_node, palette_handler =
    match Reconciler.message first with
    | Some (Wire.Message.Apply { operations; _ }) ->
      List.find_map_exn operations ~f:(function
        | Create (node, Command_palette, _, Some handler) -> Some (node, handler)
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler first |> Or_error.ok_exn;
  assert (Result.is_error (prepare (View.column [ shared ])));
  let updated =
    prepare (View.command_scope ~commands [ view (callback "next:") ]) |> Or_error.ok_exn
  in
  assert (Option.is_none (Reconciler.message updated));
  Reconciler.accept reconciler updated |> Or_error.ok_exn;
  let event reason =
    Wire.Event.Palette_dismissed (window, palette_node, palette_handler, 1L, reason)
  in
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch reconciler (event (Selected "run")))
      (Some "next:(Selected run)"));
  assert (Option.is_none (Reconciler.dispatch reconciler (event Outside_pointer)));
  let removed = prepare (View.command_scope ~commands []) |> Or_error.ok_exn in
  Reconciler.accept reconciler removed |> Or_error.ok_exn;
  assert (Option.is_none (Reconciler.dispatch reconciler (event Escape)));
  [%expect {| |}]
;;

let%expect_test "palette bytes and event order match independent Rust fixture" =
  let command_wire =
    List.mapi (Command.Registry.to_list commands) ~f:(fun index command ->
      Command.Expert.to_wire command ~generation:(Int64.of_int (index + 1)))
  in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node 0L, Command_scope, "", Some (handler 0L))
          ; Set_commands (node 0L, command_wire)
          ; Create (node 1L, Command_palette, "", Some (handler 1L))
          ; Set_palette (node 1L, Command_palette.Expert.to_wire config)
          ; Splice (node 0L, 0L, 0L, [ node 1L ])
          ; Set_root (Some (node 0L))
          ]
      }
  in
  let events =
    [ Wire.Event.Command_invoked
        (window, node 0L, handler 0L, 1L, "run", 1L, Palette (node 1L))
    ; Palette_dismissed (window, node 1L, handler 1L, 1L, Selected "run")
    ; Palette_dismissed (window, node 1L, handler 1L, 1L, Escape)
    ; Palette_dismissed (window, node 1L, handler 1L, 1L, Outside_pointer)
    ]
  in
  Eio_main.run (fun env ->
    let fixture name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun index ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(index * 2) ~len:2)))
    in
    assert (
      String.equal
        (fixture "palette-v1-request.hex")
        (Wire.Message.encode message |> Or_error.ok_exn));
    assert (
      List.equal
        Wire.Event.equal
        events
        (Wire.Event.decode (fixture "palette-v1-events.hex") |> Or_error.ok_exn)));
  [%expect {| |}]
;;

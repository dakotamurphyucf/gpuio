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

let%expect_test "palette policies validate keywords and keep default wire unchanged" =
  let create keywords =
    Command_palette.Config.create ~label:"Actions" ~commands:[ id "run" ] ~keywords ()
  in
  List.iter
    [ [ id "missing", [ "execute" ] ]
    ; [ id "run", [ "execute" ]; id "run", [ "launch" ] ]
    ; [ id "run", [ " " ] ]
    ; [ id "run", [ "a\000b" ] ]
    ; [ id "run", [ String.make 4097 'x' ] ]
    ; [ id "run", List.init 65 ~f:(Fn.const "x") ]
    ; [ id "run", List.init 64 ~f:(Fn.const (String.make 4096 'x')) ]
    ]
    ~f:(fun keywords -> assert (Result.is_error (create keywords)));
  assert (
    Result.is_ok (Command_palette.Config.create ~label:"\227\128\128" ~commands:[] ()));
  assert (
    Result.is_ok
      (create [ id "run", [ "\194\160\227\128\128"; "\194\160λ\227\128\128" ] ]));
  assert (Option.is_none (Command_palette.Expert.options config));
  let configured =
    Command_palette.Config.create
      ~label:"Actions"
      ~commands:[ id "run" ]
      ~search:Substring
      ~searchable:false
      ~escape:Clear_query_first
      ~keywords:[ id "run", [ "execute λ" ] ]
      ()
    |> Or_error.ok_exn
  in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_palette_options (node 1L, Command_palette.Expert.options configured) ]
      }
  in
  let bytes = Wire.Message.encode message |> Or_error.ok_exn in
  assert (
    String.equal
      bytes
      "\003\000\001\000\001\001\125\001\001\001\001\000\001\001\003run\001\010execute λ");
  [%expect {| |}]
;;

let%expect_test "palette option updates preserve mounted identity and reset separately" =
  let reconciler = Reconciler.create window in
  let view search =
    let config =
      Command_palette.Config.create ~label:"Actions" ~commands:[ id "run" ] ~search ()
      |> Or_error.ok_exn
    in
    View.command_scope
      ~commands
      [ View.command_palette ~config ~on_dismiss:(Fn.const "dismiss") () ]
  in
  let apply search =
    let prepared =
      Reconciler.prepare reconciler ~theme:Theme.default (Some (view search))
      |> Or_error.ok_exn
    in
    let message = Reconciler.message prepared in
    Reconciler.accept reconciler prepared |> Or_error.ok_exn;
    message
  in
  ignore (apply All_terms : Wire.Message.t option);
  let only_options = function
    | Some
        (Wire.Message.Apply { operations = [ Set_palette_options (node, options) ]; _ })
      -> node, options
    | _ -> assert false
  in
  let mounted, options = only_options (apply Substring) in
  assert (Option.is_some options);
  assert (Option.is_none (apply Substring));
  let reset, options = only_options (apply All_terms) in
  assert (Node_id.equal mounted reset);
  assert (Option.is_none options);
  [%expect {| |}]
;;

let%expect_test "palette grouped presentation validates IDs, emits layout only and resets"
  =
  let module P = Command_palette in
  let group_id = P.Group.Id.of_string "g" |> Or_error.ok_exn in
  let group =
    P.Group.create ~id:group_id ~label:"Group" ~commands:[ id "run" ] ()
    |> Or_error.ok_exn
  in
  let entries = [ P.Entry.Group group; Separator; Command (id "copy") ] in
  let grouped () =
    P.Config.create_entries ~label:"Actions" ~entries () |> Or_error.ok_exn
  in
  assert (Result.is_error (P.Group.Id.of_string " "));
  assert (Result.is_error (P.Group.create ~id:group_id ~label:"\000" ~commands:[] ()));
  assert (
    Result.is_error
      (P.Config.create_entries ~label:"Actions" ~entries:[ Group group; Group group ] ()));
  assert (
    Result.is_error
      (P.Config.create_entries
         ~label:"Actions"
         ~entries:[ Group group; Command (id "run") ]
         ()));
  assert (
    Result.is_error
      (P.Config.create_entries
         ~label:"Actions"
         ~entries:(List.init 1025 ~f:(Fn.const P.Entry.Separator))
         ()));
  assert (
    List.equal Command.Id.equal (P.Config.commands (grouped ())) [ id "run"; id "copy" ]);
  let layout = P.Expert.layout (grouped ()) in
  let fixture =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_palette_layout (node 1L, layout) ]
      }
  in
  assert (
    String.equal
      (Bin_prot.Utils.bin_dump Wire.Message.bin_writer_t fixture |> Bigstring.to_string)
      "\003\000\001\000\001\001\126\001\001\001\003\001\001g\001\005Group\001\000\002\000\001");
  let reconciler = Reconciler.create window in
  let apply config =
    let view =
      View.command_scope
        ~commands
        [ View.command_palette ~config ~on_dismiss:(Fn.const "dismiss") () ]
    in
    let prepared =
      Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
    in
    let message = Reconciler.message prepared in
    Reconciler.accept reconciler prepared |> Or_error.ok_exn;
    message
  in
  let flat =
    P.Config.create ~label:"Actions" ~commands:[ id "run"; id "copy" ] ()
    |> Or_error.ok_exn
  in
  ignore (apply flat : Wire.Message.t option);
  let only_layout = function
    | Some (Wire.Message.Apply { operations = [ Set_palette_layout (node, layout) ]; _ })
      -> node, layout
    | _ -> assert false
  in
  let mounted, layout = only_layout (apply (grouped ())) in
  assert (Option.is_some layout);
  assert (Option.is_none (apply (grouped ())));
  let reset, layout = only_layout (apply flat) in
  assert (Node_id.equal mounted reset);
  assert (Option.is_none layout);
  [%expect {| |}]
;;

let%expect_test "palette content keeps native ownership and rejects interactive rows" =
  let plain =
    View.command_palette
      ~key:(Key.of_string_exn "palette")
      ~config
      ~on_dismiss:(Fn.const "dismiss")
      ()
  in
  let button = View.button ~on_click:(fun () -> "help") "Help" in
  let rich =
    View.with_palette_content
      plain
      ~header:button
      ~footer:(View.text "Footer")
      ~empty:(View.button ~on_click:(fun () -> "retry") "Retry")
      ~items:[ id "run", View.column [ View.text "Run"; View.text "Details" ] ]
      ()
    |> Or_error.ok_exn
  in
  List.iter
    [ [ id "missing", View.text "x" ]
    ; [ id "run", button ]
    ; [ id "run", View.text "a"; id "run", View.text "b" ]
    ]
    ~f:(fun items -> assert (Result.is_error (View.with_palette_content plain ~items ())));
  assert (
    Result.is_error (View.with_palette_content (View.text "Not a palette") ~items:[] ()));
  let oversized = View.column (List.init 4096 ~f:(Fn.const (View.text "x"))) in
  assert (Result.is_error (View.with_palette_content plain ~header:oversized ~items:[] ()));
  let reconciler = Reconciler.create window in
  let commit palette =
    let prepared =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.command_scope ~commands [ palette ]))
      |> Or_error.ok_exn
    in
    let message = Reconciler.message prepared in
    Reconciler.accept reconciler prepared |> Or_error.ok_exn;
    message
  in
  ignore (commit plain : Wire.Message.t option);
  List.iter
    [ rich; View.with_palette_content rich ~items:[] () |> Or_error.ok_exn; rich ]
    ~f:(fun view ->
      match commit view with
      | Some (Apply { operations; _ }) ->
        assert (
          not
            (List.exists operations ~f:(function
               | Wire.Op.Create (_, Command_palette, _, _) | Set_palette _ -> true
               | _ -> false)))
      | _ -> assert false);
  [%expect {| |}]
;;

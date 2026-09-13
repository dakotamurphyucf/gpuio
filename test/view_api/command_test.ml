open Core
open Gpuio
open Gpuio_protocol

let id name = Command.Id.of_string name |> Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let registry ?(enabled = true) ?(label = "Run") value =
  Command.Registry.create
    [ Command.create ~id:(id "run") ~label ~enabled ~on_invoke:(fun () -> value) ()
      |> Or_error.ok_exn
    ]
  |> Or_error.ok_exn
;;

let%expect_test "command IDs, registry bounds and shortcut keys are validated" =
  List.iter [ ""; " "; "bad\000id"; "run" ] ~f:(fun name ->
    print_s [%sexp (Command.Id.of_string name |> Result.is_ok : bool)]);
  List.iter [ "enter"; "F24"; "f25"; "É"; "ab"; "\000" ] ~f:(fun key ->
    print_s [%sexp (Shortcut.create ~key () |> Result.is_ok : bool)]);
  print_s
    [%sexp
      (Shortcut.create ~key:"s" ~modifiers:[ Primary; Primary ] () |> Result.is_error
       : bool)];
  let command =
    Command.create ~id:(id "run") ~label:"Run" ~on_invoke:Fn.id () |> Or_error.ok_exn
  in
  print_s [%sexp (Command.Registry.create [ command; command ] |> Result.is_error : bool)];
  [%expect
    {|
    false
    false
    false
    true
    true
    true
    false
    true
    false
    false
    true
    true
    |}]
;;

let%expect_test
    "queued commands refresh callbacks but cannot cross disabled or removed generations"
  =
  let reconciler = Reconciler.create window in
  let button = View.command_button ~command:(id "run") () in
  let render commands =
    let view = View.command_scope ~commands [ button ] in
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
    in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let initial = render (registry "first") in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (node, Command_scope, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let generation ops =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Set_commands (_, [ command ]) -> Some command.generation
      | _ -> None)
  in
  let first = generation initial in
  let invoke generation =
    Reconciler.dispatch
      reconciler
      (Wire.Event.Command_invoked
         ( window
         , node
         , handler
         , Reconciler.revision reconciler
         , "run"
         , generation
         , Shortcut ))
  in
  print_s [%sexp (invoke first : string option)];
  print_s [%sexp (render (registry "latest") : Wire.Op.t list)];
  print_s [%sexp (invoke first : string option)];
  ignore (render (registry ~enabled:false "disabled") : Wire.Op.t list);
  print_s [%sexp (invoke first : string option)];
  let next = generation (render (registry "enabled")) in
  print_s [%sexp (Int64.(next > first) : bool)];
  print_s [%sexp (invoke first : string option)];
  print_s [%sexp (invoke next : string option)];
  ignore (render (registry ~label:"New label" "renamed") : Wire.Op.t list);
  print_s [%sexp (invoke next : string option)];
  Reconciler.close reconciler;
  print_s [%sexp (invoke next : string option)];
  [%expect
    {|
    (first)
    ()
    (latest)
    ()
    true
    ()
    (enabled)
    (renamed)
    ()
    |}]
;;

let%expect_test
    "undefined references are rejected even when a child view is physically shared"
  =
  let reconciler = Reconciler.create window in
  let button = View.command_button ~command:(id "run") () in
  let update =
    Reconciler.prepare
      reconciler
      ~theme:Theme.default
      (Some (View.command_scope ~commands:(registry ()) [ button ]))
    |> Or_error.ok_exn
  in
  Reconciler.accept reconciler update |> Or_error.ok_exn;
  let empty = Command.Registry.create [] |> Or_error.ok_exn in
  print_s
    [%sexp
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (View.command_scope ~commands:empty [ button ]))
       |> Result.is_error
       : bool)];
  print_s [%sexp (Reconciler.revision reconciler : int64)];
  let nested = View.command_scope ~commands:empty [ button ] in
  print_s
    [%sexp
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (View.command_scope ~commands:(registry ()) [ nested ]))
       |> Result.is_ok
       : bool)];
  [%expect
    {|
    true
    1
    true
    |}]
;;

let%expect_test
    "command targets, shortcut policies and invocation sources match independent Rust \
     fixtures"
  =
  let node slot = Node_id.create ~slot ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let shortcut key modifiers priority text_input during_composition =
    Shortcut.create ~key ~modifiers ~priority ~text_input ~during_composition ()
    |> Or_error.ok_exn
  in
  let callback =
    Command.create
      ~id:(id "run")
      ~label:"run"
      ~checked:false
      ~shortcuts:
        [ shortcut "k" [ Primary; Shift ] Override Always true
        ; shortcut "enter" [] Native_first Never false
        ]
      ~on_invoke:Fn.id
      ()
    |> Or_error.ok_exn
  in
  let commands =
    callback
    :: List.map
         [ "copy", Command.Native.Copy
         ; "cut", Cut
         ; "paste", Paste
         ; "select-all", Select_all
         ; "undo", Undo
         ; "redo", Redo
         ]
         ~f:(fun (label, action) ->
           Command.native
             ~id:(id label)
             ~label
             ~shortcuts:
               (if String.equal label "copy"
                then [ shortcut "c" [ Primary ] Native_first Modified_only false ]
                else [])
             action
           |> Or_error.ok_exn)
  in
  let commands =
    List.mapi commands ~f:(fun index command ->
      Command.Expert.to_wire command ~generation:(Int64.of_int (index + 1)))
  in
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node 0L, Command_scope, "", Some handler)
          ; Set_commands (node 0L, commands)
          ; Create (node 1L, Command_button, "", None)
          ; Set_command_ref (node 1L, "run")
          ; Splice (node 0L, 0L, 0L, [ node 1L ])
          ; Set_root (Some (node 0L))
          ]
      }
  in
  let events =
    List.map
      [ Wire.Command_source.Button (node 1L); Shortcut; Menu; Palette (node 2L) ]
      ~f:(fun source ->
        Wire.Event.Command_invoked (window, node 0L, handler, 1L, "run", 1L, source))
  in
  Eio_main.run (fun env ->
    let load file =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun index ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(index * 2) ~len:2)))
    in
    assert (
      String.equal
        (load "commands-v1-request.hex")
        (Wire.Message.encode request |> Or_error.ok_exn));
    let bytes = load "commands-v1-events.hex" in
    assert (
      String.equal
        bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal Wire.Event.equal events (Wire.Event.decode bytes |> Or_error.ok_exn)));
  [%expect {| |}]
;;

let%expect_test "command text blankness uses Core's ASCII whitespace definition" =
  List.iter [ ""; " "; "\t\n\011\012\r " ] ~f:(fun text ->
    assert (Result.is_error (Command.Id.of_string text)));
  List.iter [ "run"; "é界"; "\194\160"; "\226\128\131" ] ~f:(fun text ->
    assert (Result.is_ok (Command.Id.of_string text)));
  [%expect {| |}]
;;

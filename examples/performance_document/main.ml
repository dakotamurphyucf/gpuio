open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Document = Gpuio_eio.Document
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Probe = Gpuio_performance_probe
module Profile = Gpuio_example_document

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let chunk_bytes = 128 * 1024

let seed =
  "# Qualification\n\nA **native** document · λ 世界.\n\n```ocaml\nlet answer = 42\n```\n"
;;

(* Exactly 128 KiB, canonical UTF-8; the external driver independently generates
   the same content and checks native full-source copying at every final size. *)
let chunk ~document ~index =
  let header = sprintf "# Document %d / chunk %d\n\n" document index in
  let line = "Native **Markdown** and `code` · λ 世界\n" in
  let remaining = chunk_bytes - String.length header in
  header
  ^ String.concat (List.init (remaining / String.length line) ~f:(fun _ -> line))
  ^ String.make (remaining mod String.length line) ' '
;;

let perform scope ui_effect =
  let promise, resolver = Eio.Promise.create () in
  ignore
    (Scope.start
       scope
       ~f:(fun () -> ())
       ~on_result:(fun result ->
         ok result;
         E.map ui_effect ~f:(Eio.Promise.resolve resolver))
     |> ok
     : Scope.Task.t);
  Eio.Promise.await promise
;;

let component ~documents ~selected ~profile ~command ~events _window _graph =
  let open B.Let_syntax in
  let%arr documents = B.Expert.Var.value documents
  and selected = B.Expert.Var.value selected
  and profile = B.Expert.Var.value profile
  and sequence, request = B.Expert.Var.value command in
  let content =
    match documents with
    | None -> V.text "Preparing document registrations…"
    | Some documents ->
      let config =
        Gpuio.Document.Config.create
          ~source:(Document.handle documents.(selected))
          ~mode:Markdown
          ~layout:(Viewport 700.)
          ~selection_format:Markdown
          ~label:"Qualification document"
          ()
        |> ok
      in
      let view = V.document config in
      if profile
      then
        V.with_document_profile
          view
          (Profile.instance ~accent:Indigo ~generation:1L |> ok)
          ~on_event:(fun _ -> E.Ignore)
        |> ok
      else view
  in
  V.column
    ~style:
      (style
         [ Width (px 1200.)
         ; Height (px 800.)
         ; Padding (px 16.)
         ; Gap (px 8.)
         ; Background (Background.solid (Color.rgb_exn 0xf5f6f8))
         ; Foreground (Color.rgb_exn 0x182030)
         ])
    [ V.text "GPUIO qualification · growing documents / 20 MiB aggregate"
    ; content
    ; V.extension
        (Probe.instance ~sequence request |> ok)
        ~on_event:(function
          | Extension.Event.Data event ->
            E.of_thunk (fun () -> Queue.enqueue events event)
          | Failed error ->
            E.of_thunk (fun () -> raise_s [%sexp (error : Extension.Error.t)])
          | Mounted | Command_completed _ -> E.Ignore)
    ]
;;

let run ~smoke =
  let sizes = if smoke then [| 2; 2; 1 |] else [| 64; 64; 32 |] in
  App.run ~exit_on_last_window:false (fun env app ->
    let clock = Eio.Stdenv.clock env in
    let scope = App.scope app in
    let input = Eio.Buf_read.of_flow (Eio.Stdenv.stdin env) ~max_size:128 in
    let documents = B.Expert.Var.create None in
    let selected = B.Expert.Var.create 0 in
    let profile = B.Expert.Var.create true in
    let command = B.Expert.Var.create (1L, Probe.Command.Begin) in
    let events = Queue.create () in
    let window =
      App.open_window
        app
        ~focus:true
        ~title:"GPUIO · Growing document qualification"
        ~width:1200.
        ~height:800.
        (component ~documents ~selected ~profile ~command ~events)
      |> ok
    in
    let emit name payload =
      Eio.Flow.copy_string
        ("GPUIO_DOCUMENT_PERF " ^ name ^ " " ^ Sexp.to_string payload ^ "\n")
        (Eio.Stdenv.stdout env)
    in
    let sync f = perform scope (E.of_thunk f) in
    let wait label f =
      try
        Eio.Time.with_timeout_exn clock 30. (fun () ->
          while not (f ()) do
            Eio.Time.sleep clock 0.005
          done)
      with
      | Eio.Time.Timeout -> failwith ("Timed out: " ^ label)
    in
    let next_event () =
      wait "native probe" (fun () -> not (Queue.is_empty events));
      Queue.dequeue_exn events
    in
    let sequence = ref 1L in
    let send request =
      sequence := Int64.succ !sequence;
      sync (fun () -> B.Expert.Var.set command (!sequence, request))
    in
    let frame () =
      let promise, resolver = Eio.Promise.create () in
      App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
        E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
      |> ok;
      Eio.Time.with_timeout_exn clock 30. (fun () -> Eio.Promise.await promise)
    in
    let preparation name =
      send Probe.Command.Document_preparation;
      match next_event () with
      | Probe.Event.Document_preparation _ as event ->
        emit "preparation" [%sexp (name : string), (event : Probe.Event.t)]
      | event -> raise_s [%sexp (event : Probe.Event.t)]
    in
    let checkpoint_number = ref 0 in
    let checkpoint ?(on_ready = fun () -> ()) name document index =
      Int.incr checkpoint_number;
      emit
        "checkpoint"
        [%sexp
          (!checkpoint_number : int), (name : string), (document : int), (index : int)];
      if String.equal name "navigate"
      then (
        let ready =
          Eio.Time.with_timeout_exn clock 30. (fun () -> Eio.Buf_read.line input)
        in
        assert (String.equal ready (sprintf "ready %d" !checkpoint_number));
        on_ready ());
      let response =
        Eio.Time.with_timeout_exn
          clock
          (if String.equal name "copy" then 300. else 30.)
          (fun () -> Eio.Buf_read.line input)
      in
      assert (String.equal response (sprintf "continue %d" !checkpoint_number));
      frame ()
    in
    let work () =
      emit "config" [%sexp (smoke : bool), (chunk_bytes : int), (sizes : int array)];
      wait "window" (fun () -> Option.is_some (App.Window.snapshot window));
      let resources =
        Array.init 3 ~f:(fun _ ->
          match
            perform
              scope
              (Document.create
                 app
                 ~scope:(App.Window.scope window)
                 (Text_source.of_string ~status:Streaming seed |> ok))
          with
          | Ok resource -> resource
          | Error error -> raise_s [%sexp (error : Document.Error.t)])
      in
      sync (fun () -> B.Expert.Var.set documents (Some resources));
      checkpoint "initial" 0 0;
      (match next_event () with
       | Probe.Event.Begun ns -> emit "begin" [%sexp ("growth" : string), (ns : int64)]
       | event -> raise_s [%sexp (event : Probe.Event.t)]);
      preparation "before";
      Array.iteri resources ~f:(fun document resource ->
        sync (fun () -> B.Expert.Var.set selected document);
        frame ();
        Document.reset resource "" |> ok;
        wait "reset publication" (fun () -> Document.is_published resource);
        for index = 1 to sizes.(document) do
          let text = chunk ~document ~index in
          let started = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
          Document.append resource text |> ok;
          wait "append publication" (fun () ->
            Option.iter (Document.error resource) ~f:(fun error ->
              raise_s [%sexp (error : Document.Error.t)]);
            Document.is_published resource);
          let published = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
          let source = Document.source resource |> Option.value_exn in
          assert (Text_source.byte_length source = index * chunk_bytes);
          checkpoint "navigate" document index ~on_ready:(fun () ->
            let ready = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
            let ns a b = Mtime.span a b |> Mtime.Span.to_uint64_ns in
            emit
              "append"
              [%sexp
                (document : int)
              , (index : int)
              , (Text_source.byte_length source : int)
              , (ns started published : int64)
              , (ns published ready : int64)])
        done;
        if (not smoke) && document < 2
        then (
          let before = Document.source resource |> Option.value_exn in
          assert (Result.is_error (Document.append resource "x"));
          assert (Text_source.equal before (Document.source resource |> Option.value_exn));
          emit "limit-rejected" [%sexp (document : int)]);
        checkpoint "copy" document sizes.(document));
      let bytes =
        Array.fold resources ~init:0 ~f:(fun total resource ->
          total + Text_source.byte_length (Document.source resource |> Option.value_exn))
      in
      emit "retained" [%sexp (bytes : int), (App.diagnostics app : App.Diagnostics.t)];
      preparation "grown";
      sync (fun () -> B.Expert.Var.set selected 0);
      frame ();
      Document.reset resources.(0) seed |> ok;
      wait "final reset publication" (fun () -> Document.is_published resources.(0));
      checkpoint "reset" 0 0;
      sync (fun () -> B.Expert.Var.set profile false);
      frame ();
      checkpoint "profile-removed" 0 0;
      preparation "after";
      send Probe.Command.Finish;
      let finished = next_event () in
      let counts =
        match finished with
        | Probe.Event.Finished { counts; _ } -> counts
        | event -> raise_s [%sexp (event : Probe.Event.t)]
      in
      emit "finish" [%sexp ("growth" : string), (finished : Probe.Event.t)];
      List.iteri counts ~f:(fun metric _ ->
        let offset = ref 0 in
        let more = ref true in
        while !more do
          send (Probe.Command.Buckets { metric; offset = !offset });
          let event = next_event () in
          (match event with
           | Probe.Event.Buckets page ->
             assert (page.metric = metric && page.offset = !offset);
             offset := !offset + List.length page.values;
             assert (!offset <= page.total);
             more := !offset < page.total
           | event -> raise_s [%sexp (event : Probe.Event.t)]);
          emit "buckets" [%sexp ("growth" : string), (event : Probe.Event.t)]
        done);
      sync (fun () ->
        App.Window.close window;
        B.Expert.Var.set documents None);
      wait "resource retirement" (fun () ->
        let d = App.diagnostics app in
        d.windows = 0
        && d.documents = 0
        && d.document_source_bytes = 0
        && d.pending_requests = 0
        && d.queued_commands = 0
        && d.queued_jobs = 0
        && d.native_command_queue.commands = 0
        && d.native_command_queue.bytes = 0);
      emit "cleanup" [%sexp (App.diagnostics app : App.Diagnostics.t)];
      emit "complete" [%sexp (bytes : int)];
      App.shutdown app
    in
    ignore
      (Scope.start scope ~f:work ~on_result:(fun result ->
         E.of_thunk (fun () -> ok result))
       |> ok
       : Scope.Task.t))
;;

let () = run ~smoke:(Array.exists (Sys.get_argv ()) ~f:(String.equal "--smoke"))

open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Asset = Gpuio_eio.Asset
module Document = Gpuio_eio.Document
module Canvas_resource = Gpuio_eio.Canvas
module Scene = Gpuio.Canvas_scene
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Probe = Gpuio_performance_probe

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module Resources = struct
  type t =
    { image : Asset.t
    ; document : Document.t
    ; canvas : Canvas_resource.t
    }
end

let scene cycle =
  let bounds = Canvas_geometry.Rect.create ~x:16. ~y:16. ~width:200. ~height:100. |> ok in
  let paint = Scene.Paint.create ~fill:(Color.rgb_exn 0x4567da) () |> ok in
  let drawing = Scene.Drawing.rectangle bounds ~paint in
  let item = Scene.Item.create ~id:(Scene.Item_id.of_int64 1L |> ok) drawing |> ok in
  Scene.create ~description:(sprintf "Cycle %d scene" cycle) [ item ] |> ok
;;

let image_source cycle =
  (* A unique source per cycle prevents an identity cache from bypassing decode. *)
  let pixel = String.of_char (Char.of_int_exn (cycle mod 256)) ^ "\100\240" in
  Asset.Source.of_bytes
    ~format:Pnm
    ("P6\n256 256\n255\n" ^ String.concat (List.init (256 * 256) ~f:(fun _ -> pixel)))
  |> ok
;;

let component
      ~audit
      ~resources
      ~image_ready
      ~canvas_ready
      ~mounted
      ~events
      ~command
      cycle
      _window
      _graph
  =
  let open B.Let_syntax in
  let%arr resources = B.Expert.Var.value resources
  and sequence, request = B.Expert.Var.value command in
  match resources with
  | None -> V.text "Registering scoped resources…"
  | Some { Resources.image; document; canvas } ->
    let image_config =
      Image.Config.create
        ~asset:(Asset.handle image)
        ~description:(Image.Description.label "Cycle image" |> ok)
        ()
    in
    let document_config =
      Gpuio.Document.Config.create
        ~source:(Document.handle document)
        ~mode:Markdown
        ~label:"Cycle document"
        ~layout:(Viewport 180.)
        ()
      |> ok
    in
    let canvas_config =
      Canvas.Config.create
        ~scene:(Canvas_resource.handle canvas)
        ~label:"Cycle canvas"
        ~command:(Canvas.Command.create ~sequence:1L Reset_viewport |> ok)
        ()
      |> ok
    in
    let on_image state =
      E.of_thunk (fun () ->
        match state with
        | Image.State.Ready metadata ->
          assert (
            Image.Metadata.width_px metadata = 256
            && Image.Metadata.height_px metadata = 256);
          image_ready := true
        | Loading -> ()
        | Failed error -> raise_s [%sexp (error : Image.Error.t)])
    in
    let on_extension = function
      | Extension.Event.Mounted -> E.of_thunk (fun () -> mounted := true)
      | Data event -> E.of_thunk (fun () -> Queue.enqueue events event)
      | Failed error -> E.of_thunk (fun () -> raise_s [%sexp (error : Extension.Error.t)])
      | Command_completed _ -> E.Ignore
    in
    V.column
      ~style:
        (style
           [ Padding (px 16.)
           ; Gap (px 12.)
           ; Background (Background.solid (Color.rgb_exn 0xf5f6f8))
           ; Foreground (Color.rgb_exn 0x182030)
           ])
      ([ V.text (sprintf "GPUIO resource lifecycle · cycle %d" cycle)
       ; V.document document_config
       ; V.row
           [ V.image
               ~style:(style [ Width (px 256.); Height (px 256.) ])
               ~on_change:on_image
               image_config
           ; V.canvas
               ~on_event:(fun event ->
                 E.of_thunk (fun () ->
                   match event.Canvas.Event.observation with
                   | Command_completed 1L -> canvas_ready := true
                   | Failed error -> raise_s [%sexp (error : Canvas.Error.t)]
                   | Command_completed _
                   | Selection_changed _
                   | Activated _
                   | Moved _
                   | Viewport_changed _ -> ()))
               ~style:(style [ Width (px 256.); Height (px 180.) ])
               canvas_config
           ]
       ]
       @ Option.to_list
           (Option.map audit ~f:(fun audit ->
              V.extension
                ~on_event:(function
                  | Extension.Event.Failed error ->
                    E.of_thunk (fun () -> raise_s [%sexp (error : Extension.Error.t)])
                  | Mounted | Data () | Command_completed _ -> E.Ignore)
                audit))
       @ [ V.extension ~on_event:on_extension (Probe.instance ~sequence request |> ok) ])
;;

let run ~smoke ~background ~native_entities =
  let warmups, measurements = if smoke then 1, 3 else 3, 30 in
  App.run ~exit_on_last_window:false (fun env app ->
    let scope = App.scope app in
    let clock = Eio.Stdenv.clock env in
    let input = Eio.Buf_read.of_flow (Eio.Stdenv.stdin env) ~max_size:128 in
    let emit name value =
      Eio.Flow.copy_string
        ("GPUIO_LIFECYCLE " ^ name ^ " " ^ Sexp.to_string value ^ "\n")
        (Eio.Stdenv.stdout env)
    in
    let perform ui_effect =
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
    in
    let sync f = perform (E.of_thunk f) in
    let wait label f =
      try
        Eio.Time.with_timeout_exn clock 20. (fun () ->
          while not (f ()) do
            Eio.Time.sleep clock 0.005
          done)
      with
      | Eio.Time.Timeout -> failwith ("Timed out: " ^ label)
    in
    let clean () =
      let d = App.diagnostics app in
      d.windows = 0
      && d.queued_jobs = 0
      && d.queued_commands = 0
      && d.pending_requests = 0
      && d.assets = 0
      && d.asset_uploads = 0
      && d.asset_source_bytes = 0
      && d.documents = 0
      && d.document_source_bytes = 0
      && d.charts = 0
      && d.chart_data_bytes = 0
      && d.canvases = 0
      && d.canvas_scene_bytes = 0
      && d.native_command_queue.commands = 0
      && d.native_command_queue.bytes = 0
      && d.scopes.scopes = 1
      && d.scopes.tasks = 1
      && d.scopes.cleanups = 0
    in
    let work () =
      emit "config" [%sexp (warmups : int), (measurements : int), (background : bool)];
      for cycle = 1 to warmups + measurements do
        let resources = B.Expert.Var.create None in
        let command = B.Expert.Var.create (1L, Probe.Command.Begin) in
        let image_ready = ref false in
        let canvas_ready = ref false in
        let mounted = ref false in
        let events = Queue.create () in
        let window =
          sync (fun () ->
            App.open_window
              app
              ~focus:(not background)
              ~title:"GPUIO · Resource lifecycle qualification"
              ~width:1200.
              ~height:800.
              (component
                 ~audit:
                   (if native_entities
                    then
                      Some
                        (Gpuio_resource_audit.instance ~warmups ~measurements ~cycle |> ok)
                    else None)
                 ~resources
                 ~image_ready
                 ~canvas_ready
                 ~mounted
                 ~events
                 ~command
                 cycle)
            |> ok)
        in
        wait "window creation" (fun () -> Option.is_some (App.Window.snapshot window));
        let owned_scope = App.Window.scope window in
        let image =
          match perform (Asset.register app ~scope:owned_scope (image_source cycle)) with
          | Ok image -> image
          | Error error -> raise_s [%sexp (error : Asset.Error.t)]
        in
        let document =
          match
            perform (Document.create app ~scope:owned_scope (Text_source.empty_stream ()))
          with
          | Ok document -> document
          | Error error -> raise_s [%sexp (error : Document.Error.t)]
        in
        let canvas =
          match perform (Canvas_resource.create app ~scope:owned_scope (scene cycle)) with
          | Ok canvas -> canvas
          | Error error -> raise_s [%sexp (error : Canvas_resource.Error.t)]
        in
        sync (fun () ->
          B.Expert.Var.set resources (Some { Resources.image; document; canvas }));
        let text =
          sprintf
            "# Cycle %d\n\n\
             Native resources: **image**, canvas and extension.\n\n\
             ```ocaml\n\
             let message = \"世界 λ\"\n\
             ```\n"
            cycle
        in
        Document.append document text |> ok;
        Document.finish document |> ok;
        wait "image decode, document publication and extension mount" (fun () ->
          Option.iter (Document.error document) ~f:(fun error ->
            raise_s [%sexp (error : Document.Error.t)]);
          !image_ready && !canvas_ready && !mounted && Document.is_published document);
        assert (
          String.equal
            (Text_source.to_string (Option.value_exn (Document.source document)))
            text);
        let rendered, resolver = Eio.Promise.create () in
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> ok;
        Eio.Time.with_timeout_exn clock 20. (fun () -> Eio.Promise.await rendered);
        (* Even cycles complete the extension's asynchronous operation; odd
           cycles close with its delayed operation potentially still pending. *)
        if cycle mod 2 = 0
        then (
          wait "extension operation" (fun () -> not (Queue.is_empty events));
          match Queue.dequeue_exn events with
          | Probe.Event.Begun _ -> ()
          | event -> raise_s [%sexp (event : Probe.Event.t)]);
        emit "exercised" [%sexp (cycle : int), (App.diagnostics app : App.Diagnostics.t)];
        sync (fun () -> App.Window.close window);
        wait "acknowledged scope retirement" clean;
        assert (Asset.is_released image && Canvas_resource.is_released canvas);
        assert (Option.is_none (Document.source document));
        (* Drop model-held registrations before the baseline. Do not force GC
           or purge native caches to make the memory result look smaller. *)
        sync (fun () -> B.Expert.Var.set resources None);
        Eio.Time.sleep clock 2.;
        assert (clean ());
        emit "checkpoint" [%sexp (cycle : int), (App.diagnostics app : App.Diagnostics.t)];
        let response =
          Eio.Time.with_timeout_exn clock 20. (fun () -> Eio.Buf_read.line input)
        in
        assert (String.equal response (sprintf "continue %d" cycle))
      done;
      emit "complete" [%sexp (measurements : int)];
      App.shutdown app
    in
    ignore
      (Scope.start scope ~f:work ~on_result:(fun result ->
         E.of_thunk (fun () -> ok result))
       |> ok
       : Scope.Task.t))
;;

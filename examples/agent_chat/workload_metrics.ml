open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Conversation = Gpuio_agent_chat_runtime.Conversation
module Document = Gpuio_eio.Document
module Source = Gpuio.Text_source
module E = Bonsai.Effect

let output env text = Gpuio_eio.Output.write (Eio.Stdenv.stdout env) text

let start ~env ~app ~conversations =
  let app_scope = App.scope app in
  let clock = Eio.Stdenv.clock env in
  let sleep = Eio.Time.sleep clock in
  let started = Eio.Time.now clock in
  Scope.start
    app_scope
    ~f:(fun () ->
      while true do
        sleep 0.5;
        let diagnostics = App.diagnostics app in
        let response_bytes =
          List.sum
            (module Int)
            conversations
            ~f:(fun conversation ->
              Conversation.last_document conversation
              |> Option.bind ~f:Document.source
              |> Option.value_map ~default:0 ~f:Source.byte_length)
        in
        output
          env
          (sprintf
             "GPUIO_CHAT_WORKLOAD elapsed_ms=%.0f response=(response_bytes %d) \
              diagnostics=%s\n"
             ((Eio.Time.now clock -. started) *. 1000.)
             response_bytes
             (Sexp.to_string (App.Diagnostics.sexp_of_t diagnostics)))
      done)
    ~on_result:(fun result -> E.of_thunk (fun () -> Or_error.ok_exn result))
  |> Or_error.ok_exn
  |> (ignore : Scope.Task.t -> unit)
;;

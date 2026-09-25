open Core

module Config = struct
  type t =
    { chunk_bytes : int
    ; delay_seconds : float
    ; accept_delay_seconds : float
    ; fail_after_chunks : int option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(chunk_bytes = 17)
        ?(delay_seconds = 0.02)
        ?(accept_delay_seconds = 0.25)
        ?fail_after_chunks
        ()
    =
    let valid_delay value =
      Float.is_finite value && Float.(value >= 0. && value <= 10.)
    in
    if
      chunk_bytes < 1
      || chunk_bytes > 4096
      || (not (valid_delay delay_seconds && valid_delay accept_delay_seconds))
      || Option.exists fail_after_chunks ~f:(fun count -> count < 0 || count > 65536)
    then Or_error.error_string "invalid fake-backend chunk, delay or failure setting"
    else Ok { chunk_bytes; delay_seconds; accept_delay_seconds; fail_after_chunks }
  ;;

  let delay_seconds t = t.delay_seconds
  let accept_delay_seconds t = t.accept_delay_seconds
end

module Step = struct
  type t =
    | Chunk of string
    | Fail of string
    | Finish
  [@@deriving equal, sexp_of]
end

let response ~prompt =
  sprintf
    "## A native workspace\n\n\
     Your prompt contains **%d bytes**. This deterministic response demonstrates \
     streaming without a network connection.\n\n\
     1. Keep conversation work independent of its visible rows.\n\
     2. Preserve drafts while responses arrive.\n\
     3. Inspect changes before applying them.\n\n\
     ```ocaml\n\
     let greeting = \"Hello, λ 👨‍👩‍👧‍👦\"\n\
     let answer = 42\n\
     ```\n\n\
     | Check | Result |\n\
     |---|---|\n\
     | Unicode | Preserved |\n\
     | Streaming | Complete |\n\n\
     **Ready for your next question.**\n"
    (String.length prompt)
;;

let plan config ~prompt =
  let text = response ~prompt in
  let count = (String.length text + config.Config.chunk_bytes - 1) / config.chunk_bytes in
  let accepted =
    Option.value_map config.fail_after_chunks ~default:count ~f:(Int.min count)
  in
  let chunks =
    List.init accepted ~f:(fun index ->
      let pos = index * config.chunk_bytes in
      Step.Chunk
        (String.sub
           text
           ~pos
           ~len:(Int.min config.chunk_bytes (String.length text - pos))))
  in
  let terminal =
    if accepted < count
    then Step.Fail "Simulated connection interrupted. Retry to finish this response."
    else Finish
  in
  chunks @ [ terminal ]
;;

let markdown_fixture =
  "## A workspace that stays out of your way.\n\n\
   Here's a small native app with **Bonsai for state** and **GPUI for rendering**.\n\n\
   - **Room to think.** Keep a draft open while responses stream.\n\
   - **Context that travels.** Open a conversation in another window.\n\
   - **Changes you can inspect.** Review the source and proposed patch below.\n\n\
   Everything runs locally. Pick an artifact to explore it, or tell me what to build next.\n"
;;

let code_fixture =
  "open Core\n\n\
   let greeting name =\n\
  \  sprintf \"Hello, %s — λ 👨‍👩‍👧‍👦\" name\n\
   ;;\n\n\
   let answer = 42\n"
;;

let diff_fixture =
  "--- a/greeting.ml\n\
   +++ b/greeting.ml\n\
   @@ -1,2 +1,3 @@\n\
   -let answer = 0\n\
   +let answer = 42\n\
   +let status = \"ready\"\n\
  \ let language = \"OCaml\"\n"
;;
